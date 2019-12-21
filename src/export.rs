use crate::attrib::*;
use crate::material::*;
use crate::texture::*;
use gltf::json;
use json::accessor::ComponentType as GltfComponentType;
use json::accessor::Type as GltfType;
use std::mem;

use byteorder::{WriteBytesExt, LE};
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
    Standard {
        binary_path: PathBuf,
        gltf_path: PathBuf,
    },
    Binary {
        glb_path: PathBuf,
    },
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
    pub attrib_transfer: AttribTransfer,
    pub morphs: Vec<(usize, Vec<[f32; 3]>)>,
}

/// Split a sequence of keyframed trimeshes by changes in topology.
fn into_nodes(meshes: Vec<(String, usize, TriMesh, AttribTransfer)>, quiet: bool) -> Vec<Node> {
    let mut pb = ProgressBar::new(meshes.len() as u64);

    if !quiet {
        pb.message("Extracting Animation ");
    }

    let mut out = Vec::new();
    let mut mesh_iter = meshes.into_iter();

    if let Some((name, first_frame, mesh, attrib_transfer)) = mesh_iter.next() {
        out.push(Node {
            name,
            first_frame,
            mesh,
            attrib_transfer,
            morphs: Vec::new(),
        });

        while let Some((next_name, frame, next_mesh, next_attrib_transfer)) = mesh_iter.next() {
            if !quiet {
                pb.inc();
            }

            let Node {
                ref name,
                ref mesh,
                ref attrib_transfer,
                ref mut morphs,
                ..
            } = *out.last_mut().unwrap();
            if mesh.num_vertices() == next_mesh.num_vertices()
                && next_mesh.indices == mesh.indices
                && name == &next_name
                && attrib_transfer.2 == next_attrib_transfer.2
            // same material
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
                    attrib_transfer: next_attrib_transfer,
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

// The following builders are missing from gltf_json for some reason so we implement a builder here.
// These may be obsolete when the gltf crate is updated.

trait BufferViewBuilder {
    fn new(byte_length: usize, byte_offset: usize) -> json::buffer::View;
    fn with_target(self, target: json::buffer::Target) -> json::buffer::View;
    fn with_stride(self, byte_stride: usize) -> json::buffer::View;
}

impl BufferViewBuilder for json::buffer::View {
    fn new(byte_length: usize, byte_offset: usize) -> json::buffer::View {
        json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: byte_length as u32,
            byte_offset: Some(byte_offset as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: None,
        }
    }
    fn with_target(mut self, target: json::buffer::Target) -> json::buffer::View {
        self.target = Some(Valid(target));
        self
    }
    fn with_stride(mut self, byte_stride: usize) -> json::buffer::View {
        self.byte_stride = Some(byte_stride as u32);
        self
    }
}

trait AccessorBuilder {
    fn new(buf_view: usize, count: usize, generic_comp: GltfComponentType) -> json::Accessor;
    fn with_type(self, type_: GltfType) -> json::Accessor;
    fn with_component_type(
        self,
        component_type: json::accessor::GenericComponentType,
    ) -> json::Accessor;
    fn with_min_max<'a, T>(self, min: &'a [T], max: &'a [T]) -> json::Accessor
    where
        json::Value: From<&'a [T]>;
    fn with_sparse(
        self,
        count: usize,
        indices_buf_view: usize,
        values_buf_view: usize,
    ) -> json::Accessor;
}

impl AccessorBuilder for json::Accessor {
    /// Assumes scalar type.
    fn new(
        buf_view: usize,
        count: usize,
        generic_component_type: GltfComponentType,
    ) -> json::Accessor {
        // TODO: when gltf is updated to support sparse accessors without buffer view pointers,
        //       we need to replace `buffer_view` below with an Option.
        //       Probably still Some(..) since blender doesn't support proper sparse accessors.
        json::Accessor {
            buffer_view: json::Index::new(buf_view as u32).into(),
            byte_offset: 0,
            count: count as u32,
            component_type: Valid(json::accessor::GenericComponentType(generic_component_type)),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(GltfType::Scalar),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        }
    }
    fn with_type(mut self, type_: GltfType) -> json::Accessor {
        self.type_ = Valid(type_);
        self
    }
    fn with_component_type(
        mut self,
        component_type: json::accessor::GenericComponentType,
    ) -> json::Accessor {
        self.component_type = Valid(component_type);
        self
    }
    fn with_min_max<'a, T>(mut self, min: &'a [T], max: &'a [T]) -> json::Accessor
    where
        json::Value: From<&'a [T]>,
    {
        self.min = Some(json::Value::from(min));
        self.max = Some(json::Value::from(max));
        self
    }
    fn with_sparse(
        mut self,
        count: usize,
        indices_buf_view: usize,
        values_buf_view: usize,
    ) -> json::Accessor {
        self.sparse = Some(json::accessor::sparse::Sparse {
            count: count as u32,
            indices: json::accessor::sparse::Indices {
                buffer_view: json::Index::new(indices_buf_view as u32),
                byte_offset: 0,
                component_type: Valid(json::accessor::IndexComponentType(GltfComponentType::U32)),
                extensions: Default::default(),
                extras: Default::default(),
            },
            values: json::accessor::sparse::Values {
                buffer_view: json::Index::new(values_buf_view as u32),
                byte_offset: 0,
                extensions: Default::default(),
                extras: Default::default(),
            },
            extensions: Default::default(),
            extras: Default::default(),
        });
        self
    }
}

/// Generic interface to byteorder
trait WriteBytes {
    fn write_bytes(&self, data: &mut Vec<u8>);
}
impl WriteBytes for u8 {
    #[inline]
    fn write_bytes(&self, data: &mut Vec<u8>) {
        data.write_u8(*self).unwrap();
    }
}
impl WriteBytes for i8 {
    #[inline]
    fn write_bytes(&self, data: &mut Vec<u8>) {
        data.write_i8(*self).unwrap();
    }
}
impl WriteBytes for i16 {
    #[inline]
    fn write_bytes(&self, data: &mut Vec<u8>) {
        data.write_i16::<LE>(*self).unwrap();
    }
}
impl WriteBytes for u16 {
    #[inline]
    fn write_bytes(&self, data: &mut Vec<u8>) {
        data.write_u16::<LE>(*self).unwrap();
    }
}
impl WriteBytes for u32 {
    #[inline]
    fn write_bytes(&self, data: &mut Vec<u8>) {
        data.write_u32::<LE>(*self).unwrap();
    }
}
impl WriteBytes for f32 {
    #[inline]
    fn write_bytes(&self, data: &mut Vec<u8>) {
        data.write_f32::<LE>(*self).unwrap();
    }
}

macro_rules! impl_write_bytes_for_arr {
    [$($n:expr)+] => {
        $(
            impl<T: WriteBytes> WriteBytes for [T; $n] {
                #[inline]
                fn write_bytes(&self, data: &mut Vec<u8>) { for x in self { x.write_bytes(data); } }
            }
        )*
    };
}
impl_write_bytes_for_arr![2 3 4];

fn write_attribute_data<T: WriteBytes + 'static>(data: &mut Vec<u8>, attrib: &Attribute) {
    let iter = VertexAttribute::iter::<T>(&attrib.attribute).expect(&format!(
        "Unsupported attribute: \"{:?}\"",
        (attrib.name.as_str(), attrib.type_)
    ));
    iter.for_each(|x| x.write_bytes(data));
}

fn write_tex_attribute_data<T: WriteBytes + 'static>(
    data: &mut Vec<u8>,
    attrib: &TextureAttribute,
) {
    let iter = FaceVertexAttribute::iter::<T>(&attrib.attribute).expect(&format!(
        "Unsupported texture coordinate attribute: \"{:?}\"",
        (attrib.name.as_str(), attrib.component_type)
    ));
    iter.for_each(|x| x.write_bytes(data));
}

pub(crate) fn export(
    mut meshes: Vec<(String, usize, TriMesh, AttribTransfer)>,
    output: PathBuf,
    time_step: f32,
    quiet: bool,
    textures: Vec<TextureInfo>,
    materials: Vec<MaterialInfo>,
) {
    meshes.sort_by(|(name_a, frame_a, _, _), (name_b, frame_b, _, _)| {
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
        attrib_transfer,
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
        let indices_view = json::buffer::View::new(byte_length, data.len())
            .with_target(json::buffer::Target::ElementArrayBuffer);

        let mut max_index = 0;
        for idx in indices.into_iter() {
            for &i in idx.iter() {
                max_index = max_index.max(i as u32);
                data.write_u32::<LE>(i as u32).unwrap();
            }
        }

        let idx_acc = json::Accessor::new(buffer_views.len(), num_indices, GltfComponentType::U32)
            .with_min_max(&[0][..], &[max_index][..]);

        buffer_views.push(indices_view);
        let idx_acc_index = accessors.len() as u32;
        accessors.push(idx_acc);

        // Push positions to data buffer.
        let byte_length = vertex_positions.len() * mem::size_of::<[f32; 3]>();
        let pos_view = json::buffer::View::new(byte_length, data.len())
            .with_stride(mem::size_of::<[f32; 3]>())
            .with_target(json::buffer::Target::ArrayBuffer);

        let pos_view_index = buffer_views.len();
        buffer_views.push(pos_view);

        for pos in vertex_positions.iter() {
            for &coord in pos.iter() {
                data.write_f32::<LE>(coord).unwrap();
            }
        }

        let pos_acc = json::Accessor::new(
            pos_view_index,
            vertex_positions.len(),
            GltfComponentType::F32,
        )
        .with_type(GltfType::Vec3)
        .with_min_max(&bbox.min_corner()[..], &bbox.max_corner()[..]);

        let pos_acc_index = accessors.len() as u32;
        accessors.push(pos_acc);

        // Push custom vertex attributes to data buffer.
        let attrib_acc_indices: Vec<_> = attrib_transfer
            .0
            .iter()
            .map(|attrib| {
                let byte_length = attrib.attribute.buffer_ref().as_bytes().len();
                let attrib_view = json::buffer::View::new(byte_length, data.len())
                    .with_stride(call_typed_fn!(attrib.type_ => mem::size_of :: <_>()))
                    .with_target(json::buffer::Target::ArrayBuffer);

                let attrib_view_index = buffer_views.len();
                buffer_views.push(attrib_view);

                call_typed_fn!(attrib.type_ => self::write_attribute_data::<_>(&mut data, &attrib));

                let (type_, component_type) = attrib.type_.into();
                let attrib_acc =
                    json::Accessor::new(attrib_view_index, attrib.attribute.len(), component_type)
                        .with_type(type_);

                let attrib_acc_index = accessors.len() as u32;
                accessors.push(attrib_acc);
                attrib_acc_index
            })
            .collect();

        // Push texture coordinate attributes to data buffer.
        attrib_transfer.1.iter().for_each(|attrib| {
            let byte_length = attrib.attribute.buffer_ref().as_bytes().len();
            let num_bytes = match attrib.component_type {
                ComponentType::U8 => mem::size_of::<u8>(),
                ComponentType::U16 => mem::size_of::<u16>(),
                ComponentType::F32 => mem::size_of::<f32>(),
                t => panic!(
                    "Invalid texture coordinate attribute type detected: {:?}",
                    t
                ),
            };
            let attrib_view = json::buffer::View::new(byte_length, data.len())
                .with_stride(num_bytes)
                .with_target(json::buffer::Target::ArrayBuffer);

            let attrib_view_index = buffer_views.len();
            buffer_views.push(attrib_view);

            match attrib.component_type {
                ComponentType::U8 => write_tex_attribute_data::<u8>(&mut data, &attrib),
                ComponentType::U16 => write_tex_attribute_data::<u16>(&mut data, &attrib),
                ComponentType::F32 => write_tex_attribute_data::<f32>(&mut data, &attrib),
                t => panic!(
                    "Invalid texture coordinate attribute type detected: {:?}",
                    t
                ),
            }

            let attrib_acc = json::Accessor::new(
                attrib_view_index,
                attrib.attribute.len(),
                attrib.component_type.into(),
            )
            .with_type(GltfType::Vec2);

            //let attrib_acc_index = accessors.len() as u32;
            accessors.push(attrib_acc);
            //attrib_acc_index
        });

        // Initialize animation frames
        let num_animation_frames = morphs.len() + 1;

        // Sparse weight indices
        let byte_length = morphs.len() * mem::size_of::<u32>();
        let weight_indices_view = json::buffer::View::new(byte_length, data.len());

        // Note: first frame is all zeros
        for i in 0..morphs.len() {
            // all frames but first have a non-zero weight
            let index = morphs.len() * (i + 1) + i;
            data.write_u32::<LE>(index as u32).unwrap();
        }
        let weight_indices_view_index = buffer_views.len();
        buffer_views.push(weight_indices_view);

        // Initialized weights
        let byte_length = num_animation_frames * morphs.len() * mem::size_of::<f32>();
        let initial_weights_view = json::buffer::View::new(byte_length, data.len());

        for _ in 0..(num_animation_frames * morphs.len()) {
            data.write_f32::<LE>(0.0).unwrap();
        }
        let initial_weights_view_index = buffer_views.len();
        buffer_views.push(initial_weights_view);

        // Output animation frames as weights
        let weight_view = json::buffer::View::new(morphs.len() * mem::size_of::<f32>(), data.len());

        let weight_view_index = buffer_views.len();
        buffer_views.push(weight_view);

        for _ in 0..morphs.len() {
            data.write_f32::<LE>(1.0).unwrap();
        }

        // Weights accessor for all frames
        let weights_acc = json::Accessor::new(
            initial_weights_view_index,
            num_animation_frames * morphs.len(),
            GltfComponentType::F32,
        )
        .with_min_max(&[0.0][..], &[1.0][..])
        .with_sparse(morphs.len(), weight_indices_view_index, weight_view_index);

        let weights_acc_index = accessors.len() as u32;
        accessors.push(weights_acc);

        // Animation keyframe times
        let byte_length = num_animation_frames * mem::size_of::<f32>();
        let time_view = json::buffer::View::new(byte_length, data.len());

        let mut min_time = first_frame as f32 * time_step;
        let mut max_time = first_frame as f32 * time_step;
        data.write_f32::<LE>(first_frame as f32 * time_step)
            .unwrap();
        for (frame, _) in morphs.iter() {
            let time = *frame as f32 * time_step;
            min_time = min_time.min(time);
            max_time = max_time.max(time);
            data.write_f32::<LE>(time).unwrap();
        }
        let time_view_index = buffer_views.len();
        buffer_views.push(time_view);

        let time_acc = json::Accessor::new(
            time_view_index,
            num_animation_frames,
            GltfComponentType::F32,
        )
        .with_min_max(&[min_time][..], &[max_time][..]);

        let time_acc_index = accessors.len() as u32;
        accessors.push(time_acc);

        // Add morph targets
        let mut targets = Vec::new();
        for (_, displacements) in morphs.into_iter() {
            if !quiet {
                pb.inc();
            }
            let byte_length = displacements.len() * mem::size_of::<[f32; 3]>();

            let disp_view = json::buffer::View::new(byte_length, data.len())
                .with_stride(mem::size_of::<[f32; 3]>())
                .with_target(json::buffer::Target::ArrayBuffer);
            let disp_view_index = buffer_views.len();
            buffer_views.push(disp_view);

            let mut bbox = gut::bbox::BBox::empty();
            for disp in displacements.iter() {
                bbox.absorb(*disp);
                for &coord in disp.iter() {
                    data.write_f32::<LE>(coord).unwrap();
                }
            }

            let disp_acc =
                json::Accessor::new(disp_view_index, displacements.len(), GltfComponentType::F32)
                    .with_type(GltfType::Vec3)
                    .with_min_max(&bbox.min_corner()[..], &bbox.max_corner()[..]);
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
                // Texture coordinate attributes
                for (TextureAttribute { id, .. }, &attrib_acc_index) in
                    attrib_transfer.1.iter().zip(attrib_acc_indices.iter())
                {
                    map.insert(
                        Valid(json::mesh::Semantic::TexCoords(*id)),
                        json::Index::new(attrib_acc_index),
                    );
                }
                // Custom attributes
                for (Attribute { name, .. }, &attrib_acc_index) in
                    attrib_transfer.0.iter().zip(attrib_acc_indices.iter())
                {
                    use heck::ShoutySnakeCase;
                    let name = format!("_{}", name.to_shouty_snake_case());
                    map.insert(
                        Valid(json::mesh::Semantic::Extras(name)),
                        json::Index::new(attrib_acc_index),
                    );
                }
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

    // Populate images, samplers and textures
    let mut samplers = Vec::new();
    let mut images = Vec::new();
    let textures: Vec<_> = textures
        .into_iter()
        .filter_map(
            |TextureInfo {
                 image,
                 wrap_s,
                 wrap_t,
                 mag_filter,
                 min_filter,
             }| {
                let image = match image {
                    ImageInfo::Uri(path) => json::image::Image {
                        name: None,
                        buffer_view: None,
                        mime_type: None,
                        uri: Some(path),
                        extensions: Default::default(),
                        extras: Default::default(),
                    },
                    ImageInfo::Embed(path) => {
                        // Determine the type
                        let path = std::path::PathBuf::from(path);
                        let mime_type =
                            path.extension()
                                .and_then(|ext| ext.to_str())
                                .and_then(|ext| match ext.to_lowercase().as_str() {
                                    "jpeg" | "jpg" => Some("image/jpeg".to_string()),
                                    "png" => Some("image/png".to_string()),
                                    _ => None,
                                });

                        if mime_type.is_none() {
                            eprintln!(
                                "WARNING: Image must be in png or jpg format: {:?}. Skipping...",
                                &path
                            );
                            return None;
                        }

                        let mime_type = mime_type.unwrap();

                        // Read the image directly into the buffer.
                        if let Ok(mut file) = std::fs::File::open(&path) {
                            use std::io::Read;
                            let orig_len = data.len();
                            if let Ok(bytes_read) = file.read_to_end(&mut data) {
                                // Instead of guessing the size of the image we just wait until reading is
                                // done.
                                assert_eq!(bytes_read, data.len() - orig_len);
                                let image_view = json::buffer::View::new(bytes_read, orig_len);
                                let image_view_index = buffer_views.len();
                                buffer_views.push(image_view);
                                json::image::Image {
                                    name: None,
                                    buffer_view: json::Index::new(image_view_index as u32).into(),
                                    mime_type: json::image::MimeType(mime_type).into(),
                                    uri: None,
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                }
                            } else {
                                // Truncate the data vec back to original size to avoid corruption.
                                data.resize(orig_len, 0);
                                eprintln!(
                                    "WARNING: Failed to read image: {:?}. Skipping...",
                                    &path
                                );
                                return None;
                            }
                        } else {
                            eprintln!("WARNING: Failed to read image: {:?}. Skipping...", &path);
                            return None;
                        }
                    }
                };
                let image_index = images.len();
                images.push(image);

                let sampler = json::texture::Sampler {
                    mag_filter: mag_filter.into(),
                    min_filter: min_filter.into(),
                    wrap_s: wrap_s.into(),
                    wrap_t: wrap_t.into(),
                    name: None,
                    extensions: Default::default(),
                    extras: Default::default(),
                };
                let sampler_index = samplers.len();
                samplers.push(sampler);

                Some(json::texture::Texture {
                    source: json::Index::new(image_index as u32).into(),
                    sampler: json::Index::new(sampler_index as u32).into(),
                    name: None,
                    extensions: Default::default(),
                    extras: Default::default(),
                })
            },
        )
        .collect();

    // Populate materials
    let materials: Vec<_> = materials
        .iter()
        .map(
            |MaterialInfo {
                 name,
                 base_color,
                 base_texture,
                 metallic,
                 roughness,
             }| {
                json::Material {
                    name: name.to_owned(),
                    alpha_cutoff: json::material::AlphaCutoff(0.5),
                    alpha_mode: Valid(json::material::AlphaMode::Opaque),
                    double_sided: false,
                    pbr_metallic_roughness: json::material::PbrMetallicRoughness {
                        base_color_factor: json::material::PbrBaseColorFactor(*base_color),
                        base_color_texture: base_texture.map(|t| json::texture::Info {
                            index: json::Index::new(t.index),
                            tex_coord: t.texcoord,
                            extensions: Default::default(),
                            extras: Default::default(),
                        }),
                        metallic_factor: json::material::StrengthFactor(*metallic),
                        roughness_factor: json::material::StrengthFactor(*roughness),
                        metallic_roughness_texture: None,
                        extensions: Default::default(),
                        extras: Default::default(),
                    },
                    normal_texture: None,
                    occlusion_texture: None,
                    emissive_texture: None,
                    emissive_factor: json::material::EmissiveFactor([0.0, 0.0, 0.0]),
                    extensions: Default::default(),
                    extras: Default::default(),
                }
            },
        )
        .collect();

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
            Output::Standard { binary_path, .. } => Some(
                binary_path
                    .to_str()
                    .expect("ERROR: Path is not valid UTF-8")
                    .to_string(),
            ),
        },
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
        images,
        samplers,
        textures,
        materials,
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

            let writer =
                std::fs::File::create(glb_path).expect("ERROR: Failed to create output .glb file");
            glb.to_writer(writer)
                .expect("ERROR: Failed to output glTF binary data");
        }
        Output::Standard {
            binary_path,
            gltf_path,
        } => {
            // Output in standard format.
            // Two files will be produced: a .bin containing binary data and a json file containing
            // the json string (named as specified by the user). The base filename will be the one
            // matching the filename in the output path given.
            use std::io::Write;
            let writer = std::fs::File::create(gltf_path)
                .expect("ERROR: Failed to create output .gltf file");
            json::serialize::to_writer_pretty(writer, &root)
                .expect("ERROR: Failed to serialize glTF json");

            let bin = to_padded_byte_vector(data);
            let mut writer = std::fs::File::create(binary_path)
                .expect("ERROR: Failed to create output .bin file");
            writer
                .write_all(&bin)
                .expect("ERROR: Failed to output glTF binary data");
        }
    }
}
