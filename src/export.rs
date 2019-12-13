use gltf::json;
use std::mem;

use byteorder::{LittleEndian, WriteBytesExt};
use gut::mesh::topology::NumVertices;
use gut::mesh::vertex_positions::VertexPositions;
use gut::ops::*;
use json::validation::Checked::Valid;
use pbr::ProgressBar;
use std::borrow::Cow;
use std::path::PathBuf;

type TriMesh = gut::mesh::TriMesh<f32>;

#[derive(Clone)]
enum Output {
    Standard { binary_path: PathBuf, gltf_path: PathBuf },
    Binary { glb_path: PathBuf },
}

impl Output {
    /// Determine output type based on the output filename. If the filename extension is .bin or
    /// .gltf, then we produce glTF in the `Standard` form. If the extension is .glb, we produce
    /// the `Binary` form.
    ///
    /// If no extension is given, then `Binary` is assumed.
    fn from_ext(mut output: PathBuf) -> Self {
        let ext = output.extension();
        if ext.is_none() || ext.unwrap() == "glb" {
            output.set_extension("glb"); // In case it's not set.
            Output::Binary { glb_path: output }
        } else {
            let mut buffer_path = output.clone();
            buffer_path.set_extension("bin");
            Output::Standard {
                binary_path: buffer_path,
                gltf_path: output,
            }
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

fn align_to_multiple_of_four(n: u32) -> u32 {
    (n + 3) & !3
}

fn to_padded_byte_vector<T>(vec: Vec<T>) -> Vec<u8> {
    let byte_length = vec.len() * mem::size_of::<T>();
    let byte_capacity = vec.capacity() * mem::size_of::<T>();
    let alloc = vec.into_boxed_slice();
    let ptr = Box::<[T]>::into_raw(alloc) as *mut u8;
    let mut new_vec = unsafe { Vec::from_raw_parts(ptr, byte_length, byte_capacity) };
    while new_vec.len() % 4 != 0 {
        new_vec.push(0); // pad to multiple of four bytes
    }
    new_vec
}

struct Node {
    pub name: String,
    pub first_frame: usize,
    pub mesh: TriMesh,
    pub morphs: Vec<(usize, Vec<[f32; 3]>)>,
}

/// Split a sequence of keyframed trimeshes by changes in topology.
fn into_nodes(meshes: Vec<(String, usize, TriMesh)>, quiet: bool) -> Vec<Node> {
    let mut pb = ProgressBar::new(meshes.len() as u64);

    if !quiet {
        pb.message("Extracting Animation ");
    }

    let mut out = Vec::new();
    let mut mesh_iter = meshes.into_iter();

    if let Some((name, first_frame, mesh)) = mesh_iter.next() {
        out.push(Node {
            name,
            first_frame,
            mesh,
            morphs: Vec::new(),
        });

        while let Some((next_name, frame, next_mesh)) = mesh_iter.next() {
            if !quiet {
                pb.inc();
            }

            let Node {
                ref name,
                ref mesh,
                ref mut morphs,
                ..
            } = *out.last_mut().unwrap();
            if mesh.num_vertices() == next_mesh.num_vertices()
                && next_mesh.indices == mesh.indices
                && name == &next_name
            {
                // Same topology, convert positions to displacements.
                let displacements: Vec<_> = next_mesh
                    .vertex_position_iter()
                    .zip(mesh.vertex_position_iter())
                    .map(|(a, b)| [a[0] - b[0], a[1] - b[1], a[2] - b[2]])
                    .collect();
                morphs.push((frame, displacements));
            } else {
                // Different topology, instantiate a new mesh.
                out.push(Node {
                    name: next_name,
                    first_frame: frame,
                    mesh: next_mesh,
                    morphs: Vec::new(),
                });
            }
        }
    }
    if !quiet {
        pb.finish();
    }
    out
}

pub(crate) fn export(
    mut meshes: Vec<(String, usize, TriMesh)>,
    output: PathBuf,
    time_step: f32,
    quiet: bool,
) {
    meshes.sort_by(|(name_a, frame_a, _), (name_b, frame_b, _)| {
        // First sort by name
        name_a.cmp(name_b).then(frame_a.cmp(&frame_b))
    });

    // Convert sequence of meshes into meshes with morph targets by erasing repeating topology
    // data.
    let morphed_meshes = into_nodes(meshes, quiet);

    let count: u64 = morphed_meshes.iter().map(|m| m.morphs.len() as u64).sum();
    let mut pb = ProgressBar::new(count);
    if !quiet {
        pb.message("Constructing glTF    ");
    }

    let mut accessors = Vec::new();
    let mut buffer_views = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut animations = Vec::new();
    let mut data = Vec::<u8>::new();

    for Node {
        name,
        first_frame,
        mesh,
        morphs,
    } in morphed_meshes.into_iter()
    {
        let bbox = mesh.bounding_box();

        let TriMesh {
            vertex_positions,
            indices,
            ..
        } = mesh;

        // Push indices to data buffer.
        let num_indices = indices.len() * 3;
        let byte_length = num_indices * mem::size_of::<u32>();
        let indices_view = json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: byte_length as u32,
            byte_offset: Some(data.len() as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: Some(Valid(json::buffer::Target::ElementArrayBuffer)),
        };
        let mut max_index = 0;
        for idx in indices.into_iter() {
            for &i in idx.iter() {
                max_index = max_index.max(i as u32);
                data.write_u32::<LittleEndian>(i as u32).unwrap();
            }
        }
        let idx_acc = json::Accessor {
            buffer_view: Some(json::Index::new(buffer_views.len() as u32)),
            byte_offset: 0,
            count: num_indices as u32,
            component_type: Valid(json::accessor::GenericComponentType(
                json::accessor::ComponentType::U32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(json::accessor::Type::Scalar),
            min: Some(json::Value::from(&[0][..])),
            max: Some(json::Value::from(&[max_index][..])),
            name: None,
            normalized: false,
            sparse: None,
        };
        buffer_views.push(indices_view);
        let idx_acc_index = accessors.len() as u32;
        accessors.push(idx_acc);

        // Push positions to data buffer.
        let byte_length = vertex_positions.len() * mem::size_of::<[f32; 3]>();
        let pos_view = json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: byte_length as u32,
            byte_offset: Some(data.len() as u32),
            byte_stride: Some(mem::size_of::<[f32; 3]>() as u32),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: Some(Valid(json::buffer::Target::ArrayBuffer)),
        };
        let pos_view_index = buffer_views.len();
        buffer_views.push(pos_view);

        for pos in vertex_positions.iter() {
            for &coord in pos.iter() {
                data.write_f32::<LittleEndian>(coord).unwrap();
            }
        }

        let pos_acc = json::Accessor {
            buffer_view: Some(json::Index::new(pos_view_index as u32)),
            byte_offset: 0,
            count: vertex_positions.len() as u32,
            component_type: Valid(json::accessor::GenericComponentType(
                json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(json::accessor::Type::Vec3),
            min: Some(json::Value::from(&bbox.min_corner()[..])),
            max: Some(json::Value::from(&bbox.max_corner()[..])),
            name: None,
            normalized: false,
            sparse: None,
        };
        let pos_acc_index = accessors.len() as u32;
        accessors.push(pos_acc);

        // Initialize animation frames

        let num_animation_frames = morphs.len() + 1;

        // Sparse weight indices
        let byte_length = morphs.len() * mem::size_of::<u32>();
        let weight_indices_view = json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: byte_length as u32,
            byte_offset: Some(data.len() as u32),
            byte_stride: None, //Some(mem::size_of::<u32>() as u32),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: None,
        };
        // Note: first frame is all zeros
        for i in 0..morphs.len() {
            // all frames but first have a non-zero weight
            let index = morphs.len() * (i + 1) + i;
            data.write_u32::<LittleEndian>(index as u32).unwrap();
        }
        let weight_indices_view_index = buffer_views.len();
        buffer_views.push(weight_indices_view);

        // Initialized weights
        let initial_weights_view_index = if cfg!(feature = "empty-sparse-base-buffer-view") {
            None
        } else {
            let byte_length = num_animation_frames * morphs.len() * mem::size_of::<f32>();
            let initial_weights_view = json::buffer::View {
                buffer: json::Index::new(0),
                byte_length: byte_length as u32,
                byte_offset: Some(data.len() as u32),
                byte_stride: None,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: None,
            };
            for _ in 0..(num_animation_frames * morphs.len()) {
                data.write_f32::<LittleEndian>(0.0).unwrap();
            }
            let initial_weights_view_index = Some(json::Index::new(buffer_views.len() as u32));
            buffer_views.push(initial_weights_view);
            initial_weights_view_index
        };

        // Output animation frames as weights
        let weight_view = json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: (morphs.len() * mem::size_of::<f32>()) as u32,
            byte_offset: Some(data.len() as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: None,
        };
        let weight_view_index = buffer_views.len();
        buffer_views.push(weight_view);

        for _ in 0..morphs.len() {
            data.write_f32::<LittleEndian>(1.0).unwrap();
        }

        // Weights accessor for all frames
        let weights_acc = json::Accessor {
            buffer_view: initial_weights_view_index,
            byte_offset: 0,
            count: (num_animation_frames * morphs.len()) as u32,
            component_type: Valid(json::accessor::GenericComponentType(
                json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(json::accessor::Type::Scalar),
            min: Some(json::Value::from(&[0.0][..])),
            max: Some(json::Value::from(&[1.0][..])),
            name: None,
            normalized: false,
            sparse: Some(json::accessor::sparse::Sparse {
                count: morphs.len() as u32,
                indices: json::accessor::sparse::Indices {
                    buffer_view: json::Index::new(weight_indices_view_index as u32),
                    byte_offset: 0,
                    component_type: Valid(json::accessor::IndexComponentType(
                        json::accessor::ComponentType::U32,
                    )),
                    extensions: Default::default(),
                    extras: Default::default(),
                },
                values: json::accessor::sparse::Values {
                    buffer_view: json::Index::new(weight_view_index as u32),
                    byte_offset: 0,
                    extensions: Default::default(),
                    extras: Default::default(),
                },
                extensions: Default::default(),
                extras: Default::default(),
            }),
        };
        let weights_acc_index = accessors.len() as u32;
        accessors.push(weights_acc);

        // Animation keyframe times
        let byte_length = num_animation_frames * mem::size_of::<f32>();
        let time_view = json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: byte_length as u32,
            byte_offset: Some(data.len() as u32),
            byte_stride: None, //Some(mem::size_of::<f32>() as u32),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: None,
        };

        let mut min_time = first_frame as f32 * time_step;
        let mut max_time = first_frame as f32 * time_step;
        data.write_f32::<LittleEndian>(first_frame as f32 * time_step)
            .unwrap();
        for (frame, _) in morphs.iter() {
            let time = *frame as f32 * time_step;
            min_time = min_time.min(time);
            max_time = max_time.max(time);
            data.write_f32::<LittleEndian>(time).unwrap();
        }
        let time_view_index = buffer_views.len();
        buffer_views.push(time_view);

        let time_acc = json::Accessor {
            buffer_view: Some(json::Index::new(time_view_index as u32)),
            byte_offset: 0,
            count: num_animation_frames as u32,
            component_type: Valid(json::accessor::GenericComponentType(
                json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(json::accessor::Type::Scalar),
            min: Some(json::Value::from(&[min_time][..])),
            max: Some(json::Value::from(&[max_time][..])),
            name: None,
            normalized: false,
            sparse: None,
        };
        let time_acc_index = accessors.len() as u32;
        accessors.push(time_acc);

        // Add morph targets
        let mut targets = Vec::new();
        for (_, displacements) in morphs.into_iter() {
            if !quiet {
                pb.inc();
            }
            let byte_length = displacements.len() * mem::size_of::<[f32; 3]>();
            let disp_view = json::buffer::View {
                buffer: json::Index::new(0),
                byte_length: byte_length as u32,
                byte_offset: Some(data.len() as u32),
                byte_stride: Some(mem::size_of::<[f32; 3]>() as u32),
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(json::buffer::Target::ArrayBuffer)),
            };
            let disp_view_index = buffer_views.len();
            buffer_views.push(disp_view);

            let mut bbox = gut::bbox::BBox::empty();
            for disp in displacements.iter() {
                bbox.absorb(*disp);
                for &coord in disp.iter() {
                    data.write_f32::<LittleEndian>(coord).unwrap();
                }
            }

            let disp_acc = json::Accessor {
                buffer_view: Some(json::Index::new(disp_view_index as u32)),
                byte_offset: 0,
                count: displacements.len() as u32,
                component_type: Valid(json::accessor::GenericComponentType(
                    json::accessor::ComponentType::F32,
                )),
                extensions: Default::default(),
                extras: Default::default(),
                type_: Valid(json::accessor::Type::Vec3),
                min: Some(json::Value::from(&bbox.min_corner()[..])),
                max: Some(json::Value::from(&bbox.max_corner()[..])),
                name: None,
                normalized: false,
                sparse: None,
            };
            let disp_acc_index = accessors.len() as u32;
            accessors.push(disp_acc);

            targets.push(json::mesh::MorphTarget {
                positions: Some(json::Index::new(disp_acc_index)),
                normals: None,
                tangents: None,
            });
        }

        // Add an animation using this morph target
        let channel = json::animation::Channel {
            sampler: json::Index::new(0),
            target: json::animation::Target {
                path: Valid(json::animation::Property::MorphTargetWeights),
                node: json::Index::new(nodes.len() as u32),
                extensions: Default::default(),
                extras: Default::default(),
            },
            extensions: Default::default(),
            extras: Default::default(),
        };

        let sampler = json::animation::Sampler {
            input: json::Index::new(time_acc_index), // time
            interpolation: Valid(json::animation::Interpolation::Linear),
            output: json::Index::new(weights_acc_index), // weights
            extensions: Default::default(),
            extras: Default::default(),
        };
        animations.push(json::Animation {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            channels: vec![channel],
            samplers: vec![sampler],
        });

        let primitives = vec![json::mesh::Primitive {
            attributes: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    Valid(json::mesh::Semantic::Positions),
                    json::Index::new(pos_acc_index),
                );
                map
            },
            extensions: Default::default(),
            extras: Default::default(),
            indices: Some(json::Index::new(idx_acc_index)),
            material: None,
            mode: Valid(json::mesh::Mode::Triangles),
            targets: Some(targets),
        }];

        nodes.push(json::Node {
            camera: None,
            children: None,
            extensions: Default::default(),
            extras: Default::default(),
            matrix: None,
            mesh: Some(json::Index::new(meshes.len() as u32)),
            name: Some(name),
            rotation: None,
            scale: None,
            translation: None,
            skin: None,
            weights: None,
        });

        meshes.push(json::Mesh {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            primitives,
            weights: None,
        });
    }

    if !quiet {
        pb.finish();
        println!("Writing glTF to File...");
    }

    let output = Output::from_ext(output);

    let buffer = json::Buffer {
        byte_length: data.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: match &output {
            Output::Binary { .. } => None,
            Output::Standard { binary_path, .. } => Some(binary_path.to_str().expect("ERROR: Path is not valid UTF-8").to_string()),
        }
    };

    let num_nodes = nodes.len();

    let root = json::Root {
        asset: json::Asset {
            generator: Some(format!("gltfgen v{}", structopt::clap::crate_version!())),
            ..Default::default()
        },
        animations,
        accessors,
        buffers: vec![buffer],
        buffer_views,
        meshes,
        nodes,
        scenes: vec![json::Scene {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            nodes: (0..num_nodes).map(|i| json::Index::new(i as u32)).collect(),
        }],
        ..Default::default()
    };

    match output {
        Output::Binary { glb_path } => {
            // Output in binary format.
            let json_string =
                json::serialize::to_string(&root).expect("ERROR: Failed to serialize glTF json");
            let json_offset = align_to_multiple_of_four(json_string.len() as u32);

            let glb = gltf::binary::Glb {
                header: gltf::binary::Header {
                    magic: b"glTF".clone(),
                    version: 2,
                    length: json_offset + align_to_multiple_of_four(data.len() as u32),
                },
                bin: Some(Cow::Owned(to_padded_byte_vector(data))),
                json: Cow::Owned(json_string.into_bytes()),
            };

            let writer = std::fs::File::create(glb_path).expect("ERROR: Failed to create output .glb file");
            glb.to_writer(writer)
                .expect("ERROR: Failed to output glTF binary data");
        }
        Output::Standard { binary_path, gltf_path } => {
            // Output in standard format.
            // Two files will be produced: a .bin containing binary data and a json file containing
            // the json string (named as specified by the user). The base filename will be the one
            // matching the filename in the output path given.
            use std::io::Write;
            let writer = std::fs::File::create(gltf_path).expect("ERROR: Failed to create output .gltf file");
            json::serialize::to_writer_pretty(writer, &root).expect("ERROR: Failed to serialize glTF json");

            let bin = to_padded_byte_vector(data);
            let mut writer = std::fs::File::create(binary_path).expect("ERROR: Failed to create output .bin file");
            writer.write_all(&bin).expect("ERROR: Failed to output glTF binary data");
        }
    }
}
