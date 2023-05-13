use std::borrow::Cow;
use std::mem;
use std::path::PathBuf;

use byteorder::{WriteBytesExt, LE};
use gltf::json;
use json::accessor::ComponentType as GltfComponentType;
use json::accessor::Type as GltfType;
use json::validation::Checked::Valid; // For colouring log messages.

use meshx::mesh::vertex_positions::VertexPositions;

mod animation;
mod builders;
mod primitives;

use animation::*;
pub(crate) use builders::*;
use meshx::ops::BoundingBox;
use num_traits::ToPrimitive;
use primitives::*;

use crate::attrib::*;
use crate::material::*;
use crate::mesh::Mesh;
use crate::texture::*;
use crate::utils::*;

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
    pub mesh: Mesh,
    pub attrib_transfer: AttribTransfer,
    pub morphs: Vec<(usize, Vec<[f32; 3]>)>,
}

/// Split a sequence of keyframed trimeshes by changes in topology.
fn into_nodes(meshes: Vec<(String, usize, Mesh, AttribTransfer)>, quiet: bool) -> Vec<Node> {
    let pb = new_progress_bar(quiet, meshes.len());
    pb.set_message("Extracting Animation");

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

        for (next_name, frame, next_mesh, next_attrib_transfer) in mesh_iter {
            pb.tick();
            let Node {
                ref name,
                ref mesh,
                ref attrib_transfer,
                ref mut morphs,
                ..
            } = *out.last_mut().unwrap();
            if mesh.eq_topo(&next_mesh)
                && name == &next_name
                && attrib_transfer.material_ids == next_attrib_transfer.material_ids
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

    pb.finish_with_message("Done extracting animation");
    out
}

struct TextureData {
    samplers: Vec<json::texture::Sampler>,
    images: Vec<json::image::Image>,
    textures: Vec<json::texture::Texture>,
}

fn process_auto_textures(textures: &mut [TextureInfo], output: &Output) {
    // Process auto textures.
    for TextureInfo { image, .. } in textures.iter_mut() {
        if let ImageInfo::Auto(path) = image {
            match output {
                Output::Binary { .. } => *image = ImageInfo::Embed(path.clone()),
                Output::Standard { .. } => *image = ImageInfo::Uri(path.clone()),
            }
        };
    }
}

fn build_texture_data(
    textures: Vec<TextureInfo>,
    data: &mut Vec<u8>,
    buffer_views: &mut Vec<json::buffer::View>,
    warnings: &mut Vec<(usize, String)>,
) -> TextureData {
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
                            log!(warnings;
                                "Image must be in png or jpg format: {:?}. Skipping...",
                                &path
                            );
                            return None;
                        }

                        let mime_type = mime_type.unwrap();

                        // Read the image directly into the buffer.
                        if let Ok(mut file) = std::fs::File::open(&path) {
                            use std::io::Read;
                            let orig_len = data.len();
                            if let Ok(bytes_read) = file.read_to_end(data) {
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
                                log!(warnings;
                                    "Failed to read image: {:?}. Skipping...",
                                    &path
                                );
                                return None;
                            }
                        } else {
                            log!(warnings;
                                "Failed to read image: {:?}. Skipping...",
                                &path
                            );
                            return None;
                        }
                    }
                    ImageInfo::Auto(path) => unreachable!("Unexpected Auto({path}) image. All images should be converted to either Embed or Uri."),
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
                    source: json::Index::new(image_index as u32),
                    sampler: json::Index::new(sampler_index as u32).into(),
                    name: None,
                    extensions: Default::default(),
                    extras: Default::default(),
                })
            },
        )
        .collect();

    TextureData {
        samplers,
        images,
        textures,
    }
}

/// Loads local materials and textures from from attrib transfer into the global materials and textures arrays.
///
/// This function also promotes local materials in attrib_transfer to global, so only MaterialIds::Global variants need to be handled downstream.
fn extract_local_materials_and_textures(
    attrib_transfer: &mut AttribTransfer,
    materials: &mut Vec<MaterialInfo>,
    textures: &mut Vec<TextureInfo>,
) {
    if let Some(MaterialIds::Local { map }) = &mut attrib_transfer.material_ids {
        let mut global_map = indexmap::IndexMap::new();
        for (mtl, indices) in map.iter_mut() {
            let orig_indices = global_map
                .entry(materials.len().to_u32().expect(
                    "Number of materials loaded does not fit into a 32 bit unsigned integer.",
                ))
                .or_insert_with(Vec::new);
            orig_indices.append(indices);

            let mut mtl_info = MaterialInfo::from(mtl);

            // If there is a texture specified and we can find a texture
            // coordinate attribute, add to the TextureInfo vector.
            if let Some(texture_path) = &mtl.map_kd {
                // Use the first texture attrib if it exists
                if !attrib_transfer.tex_attribs_to_keep.is_empty() {
                    mtl_info.base_texture = TextureRef::Some {
                        index: textures.len().to_u32().expect("Number of textures loaded does not fit into a 32 bit unsigned integer."), // New texture added below
                        texcoord: 0,
                    };
                }
                textures.push(TextureInfo {
                    image: ImageInfo::Auto(texture_path.clone()),
                    ..Default::default()
                });
            }
            materials.push(mtl_info);
        }
        // Local materials promoted to global, save them as such.
        attrib_transfer.material_ids = Some(MaterialIds::Global { map: global_map });
    }
}

pub fn export(
    mut meshes: Vec<(String, usize, Mesh, AttribTransfer)>,
    output: PathBuf,
    time_step: f32,
    quiet: bool,
    mut textures: Vec<TextureInfo>,
    mut materials: Vec<MaterialInfo>,
) {
    meshes.sort_by(|(name_a, frame_a, _, _), (name_b, frame_b, _, _)| {
        // First sort by name
        name_a.cmp(name_b).then(frame_a.cmp(frame_b))
    });

    // Convert sequence of meshes into meshes with morph targets by erasing repeating topology
    // data.
    let mut morphed_meshes = into_nodes(meshes, quiet);

    // Load local materials from loaded objs into our configuration array.
    for Node {
        ref mut attrib_transfer,
        ..
    } in morphed_meshes.iter_mut()
    {
        extract_local_materials_and_textures(attrib_transfer, &mut materials, &mut textures);
    }

    // Convert to const binding, guarantess no further modifications.
    let morphed_meshes = morphed_meshes;

    let count: u64 = morphed_meshes.iter().map(|m| m.morphs.len() as u64).sum();
    let pb = new_progress_bar(quiet, count as usize);
    pb.set_message("Constructing glTF");

    // Keep track of the messages and warnings to be displayed after construction is complete.
    let mut msgs = Vec::new();
    let mut warnings = Vec::new();

    // First populate materials
    // Doing this first allows us to attach a default material if one is needed.
    let mut materials: Vec<_> = materials.into_iter().map(Into::into).collect();

    let mut accessors = Vec::new();
    let mut buffer_views = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut animation_channels = Vec::new();
    let mut animation_samplers = Vec::new();
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

        let (vertex_positions, indices) = mesh.build_topology(
            &attrib_transfer,
            &mut data,
            &mut buffer_views,
            &mut accessors,
        );

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

        let pos_acc = json::Accessor::new(vertex_positions.len(), GltfComponentType::F32)
            .with_buffer_view(pos_view_index)
            .with_type(GltfType::Vec3)
            .with_min_max(&bbox.min_corner()[..], &bbox.max_corner()[..]);

        let pos_acc_index = accessors.len() as u32;
        accessors.push(pos_acc);

        // Push color vertex attribute
        let color_attrib_acc_indices: Vec<_> = attrib_transfer
            .color_attribs_to_keep
            .iter()
            .filter_map(|attrib| {
                let num_bytes = match attrib.type_ {
                    Type::Vec3(ComponentType::U8) => mem::size_of::<[u8; 3]>(),
                    Type::Vec3(ComponentType::U16) => mem::size_of::<[u16; 3]>(),
                    Type::Vec3(ComponentType::F32) => mem::size_of::<[f32; 3]>(),
                    Type::Vec4(ComponentType::U8) => mem::size_of::<[u8; 4]>(),
                    Type::Vec4(ComponentType::U16) => mem::size_of::<[u16; 4]>(),
                    Type::Vec4(ComponentType::F32) => mem::size_of::<[f32; 4]>(),
                    t => {
                        log!(warnings;
                            "Invalid color attribute type detected: {:?}. Skipping...",
                            t
                        );
                        return None;
                    }
                };
                let byte_length = attrib.attribute.len() * num_bytes;

                let attrib_view = json::buffer::View::new(byte_length, data.len())
                    .with_stride(num_bytes)
                    .with_target(json::buffer::Target::ArrayBuffer);

                let attrib_view_index = buffer_views.len();
                buffer_views.push(attrib_view);

                match attrib.type_ {
                    Type::Vec3(ComponentType::U8) => {
                        write_color_attribute_data::<[u8; 3]>(&mut data, attrib)
                    }
                    Type::Vec3(ComponentType::U16) => {
                        write_color_attribute_data::<[u16; 3]>(&mut data, attrib)
                    }
                    Type::Vec3(ComponentType::F32) => {
                        write_color_attribute_data::<[f32; 3]>(&mut data, attrib)
                    }
                    Type::Vec4(ComponentType::U8) => {
                        write_color_attribute_data::<[u8; 4]>(&mut data, attrib)
                    }
                    Type::Vec4(ComponentType::U16) => {
                        write_color_attribute_data::<[u16; 4]>(&mut data, attrib)
                    }
                    Type::Vec4(ComponentType::F32) => {
                        write_color_attribute_data::<[f32; 4]>(&mut data, attrib)
                    }
                    // This must have been checked above.
                    _ => unreachable!(),
                }

                let (type_, component_type) = attrib.type_.into();
                let attrib_acc = json::Accessor::new(attrib.attribute.len(), component_type)
                    .with_name(attrib.name.clone())
                    .with_buffer_view(attrib_view_index)
                    .with_type(type_);

                let attrib_acc_index = accessors.len() as u32;
                accessors.push(attrib_acc);
                Some(attrib_acc_index)
            })
            .collect();

        // Push custom vertex attributes to data buffer.
        let attrib_acc_indices: Vec<_> = attrib_transfer
            .attribs_to_keep
            .iter()
            .map(|attrib| {
                let byte_length = attrib.attribute.data.direct_data().unwrap().byte_len();
                let attrib_view = json::buffer::View::new(byte_length, data.len())
                    .with_stride(call_typed_fn!(attrib.type_ => mem::size_of :: <_>()))
                    .with_target(json::buffer::Target::ArrayBuffer);

                let attrib_view_index = buffer_views.len();
                buffer_views.push(attrib_view);

                call_typed_fn!(attrib.type_ => self::write_attribute_data::<_>(&mut data, attrib));

                let (type_, component_type) = attrib.type_.into();
                let attrib_acc = json::Accessor::new(attrib.attribute.len(), component_type)
                    .with_name(attrib.name.clone())
                    .with_buffer_view(attrib_view_index)
                    .with_type(type_);

                let attrib_acc_index = accessors.len() as u32;
                accessors.push(attrib_acc);
                attrib_acc_index
            })
            .collect();

        // Push texture coordinate attributes to data buffer.
        let tex_attrib_acc_indices: Vec<_> = attrib_transfer
            .tex_attribs_to_keep
            .iter()
            .filter_map(|attrib| {
                let byte_length = attrib.attribute.data.direct_data().unwrap().byte_len();
                let num_bytes = match attrib.component_type {
                    ComponentType::U8 => mem::size_of::<[u8; 2]>(),
                    ComponentType::U16 => mem::size_of::<[u16; 2]>(),
                    ComponentType::F32 => mem::size_of::<[f32; 2]>(),
                    t => {
                        log!(warnings;
                            "Invalid texture coordinate attribute type detected: {:?}. Skipping...",
                            t
                        );
                        return None;
                    }
                };
                let orig_data_len = data.len();

                // First let's try to write the data to flush out any problems before appending the
                // buffer view. This way we can bail early without having to roll back state.
                match attrib.component_type {
                    ComponentType::U8 => write_tex_attribute_data::<u8>(&mut data, attrib),
                    ComponentType::U16 => write_tex_attribute_data::<u16>(&mut data, attrib),
                    ComponentType::F32 => write_tex_attribute_data::<f32>(&mut data, attrib),
                    // Other cases must have caused a return in the match above.
                    _ => {
                        unreachable!()
                    }
                }

                // Everything seems ok, continue with building the json structure.
                let attrib_view = json::buffer::View::new(byte_length, orig_data_len)
                    .with_stride(num_bytes)
                    .with_target(json::buffer::Target::ArrayBuffer);

                let attrib_view_index = buffer_views.len();
                buffer_views.push(attrib_view);

                let attrib_acc =
                    json::Accessor::new(attrib.attribute.len(), attrib.component_type.into())
                        .with_name(attrib.name.clone())
                        .with_buffer_view(attrib_view_index)
                        .with_type(GltfType::Vec2);

                let attrib_acc_index = accessors.len() as u32;
                accessors.push(attrib_acc);
                Some(attrib_acc_index)
            })
            .collect();

        // If colors or textures were specified but not materials, add a default material.
        if (!attrib_transfer.color_attribs_to_keep.is_empty()
            || !attrib_transfer.tex_attribs_to_keep.is_empty())
            && materials.is_empty()
        {
            materials.push(MaterialInfo::default().into());
        }

        let targets = build_animation(
            first_frame,
            &morphs,
            nodes.len(),
            &mut accessors,
            &mut buffer_views,
            &mut data,
            time_step,
            &pb,
        )
        .map(|(mut channel, sampler, targets)| {
            // Override the sampler index to correspond to the index within the animation_samplers Vec.
            channel.sampler = json::Index::new(animation_samplers.len() as u32);
            animation_channels.push(channel);
            animation_samplers.push(sampler);
            targets
        });

        let mode = Valid(if indices.is_some() {
            json::mesh::Mode::Triangles
        } else {
            json::mesh::Mode::Points
        });

        let primitives = build_primitives(
            mode,
            pos_acc_index,
            &attrib_transfer,
            &attrib_acc_indices,
            &color_attrib_acc_indices,
            &tex_attrib_acc_indices,
            indices,
            targets,
            materials.len(),
            &mut msgs,
        );

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

    let animations = if !animation_channels.is_empty() {
        vec![json::Animation {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            channels: animation_channels,
            samplers: animation_samplers,
        }]
    } else {
        vec![]
    };

    let output = Output::from_ext(output);

    // Convert auto texture images to embedded or uri based on selected output.
    process_auto_textures(&mut textures, &output);

    let TextureData {
        samplers,
        images,
        textures,
    } = build_texture_data(textures, &mut data, &mut buffer_views, &mut warnings);

    pb.finish_with_message("Done constructing glTF");

    // Print all accumulated warnings and messages.
    print_info(msgs);
    print_warnings(warnings);

    let buffer = json::Buffer {
        byte_length: data.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: match &output {
            Output::Binary { .. } => None,
            Output::Standard { binary_path, .. } => Some(format!(
                "./{}",
                binary_path
                    .file_name()
                    .unwrap_or_else(|| panic!(
                        "ERROR: Invalid binary path: {}",
                        binary_path.display()
                    ))
                    .to_str()
                    .expect("ERROR: Path is not valid UTF-8")
            )),
        },
    };

    let num_nodes = nodes.len();

    let root = json::Root {
        asset: json::Asset {
            generator: Some(format!("gltfgen v{}", clap::crate_version!())),
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

    let pb = new_progress_bar_file(quiet, 0);
    pb.set_message("Writing glTF to File");

    match output {
        Output::Binary { glb_path } => {
            // Output in binary format.
            let json_string =
                json::serialize::to_string(&root).expect("ERROR: Failed to serialize glTF json");
            let json_offset = align_to_multiple_of_four(json_string.len() as u32);

            let glb = gltf::binary::Glb {
                header: gltf::binary::Header {
                    magic: *b"glTF",
                    version: 2,
                    length: json_offset + align_to_multiple_of_four(data.len() as u32),
                },
                bin: Some(Cow::Owned(to_padded_byte_vector(data))),
                json: Cow::Owned(json_string.into_bytes()),
            };

            // This is an approximation of the total size.
            pb.set_length((glb.header.length + 28) as u64);

            let writer =
                std::fs::File::create(glb_path).expect("ERROR: Failed to create output .glb file");
            glb.to_writer(pb.wrap_write(writer))
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

            pb.set_length(bin.len() as u64);

            let writer = std::fs::File::create(binary_path)
                .expect("ERROR: Failed to create output .bin file");
            pb.wrap_write(writer)
                .write_all(&bin)
                .expect("ERROR: Failed to output glTF binary data");
        }
    }

    pb.finish_with_message("Success!");
}
