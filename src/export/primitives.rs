use crate::AttribTransfer;
use crate::Attribute;
use crate::MaterialIds;
use crate::TextureAttribute;

use gltf::json;
use gltf::json::validation::Checked;
use json::validation::Checked::Valid;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_primitives(
    mode: Checked<json::mesh::Mode>,
    pos_acc_index: u32,
    attrib_transfer: &AttribTransfer,
    attrib_acc_indices: &[u32],
    color_attrib_acc_indices: &[u32],
    tex_attrib_acc_indices: &[u32],
    indices: Option<Vec<json::Index<json::Accessor>>>,
    targets: Option<Vec<json::mesh::MorphTarget>>,
    num_materials: usize,
    msgs: &mut Vec<(usize, String)>,
) -> Vec<json::mesh::Primitive> {
    // TODO: Split the mesh into multiple primitives, one for each material that appears on the mesh.
    let build_attributes = || {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            Valid(json::mesh::Semantic::Positions),
            json::Index::new(pos_acc_index),
        );
        // Color attributes
        for (id, (Attribute { .. }, &attrib_acc_index)) in attrib_transfer
            .color_attribs_to_keep
            .iter()
            .zip(color_attrib_acc_indices.iter())
            .enumerate()
        {
            map.insert(
                Valid(json::mesh::Semantic::Colors(id as u32)),
                json::Index::new(attrib_acc_index),
            );
        }
        // Texture coordinate attributes
        for (TextureAttribute { id, .. }, &attrib_acc_index) in attrib_transfer
            .tex_attribs_to_keep
            .iter()
            .zip(tex_attrib_acc_indices.iter())
        {
            map.insert(
                Valid(json::mesh::Semantic::TexCoords(*id)),
                json::Index::new(attrib_acc_index),
            );
        }
        // Custom attributes
        for (Attribute { name, .. }, &attrib_acc_index) in attrib_transfer
            .attribs_to_keep
            .iter()
            .zip(attrib_acc_indices.iter())
        {
            use heck::ToShoutySnakeCase;
            map.insert(
                Valid(json::mesh::Semantic::Extras(name.to_shouty_snake_case())),
                json::Index::new(attrib_acc_index),
            );
        }
        map
    };

    if let Some(indices) = indices {
        if let Some(MaterialIds::Global { map }) = &attrib_transfer.material_ids {
            indices
                .into_iter()
                .zip(map.keys())
                .map(|(indices, &mtl_id)| json::mesh::Primitive {
                    attributes: build_attributes(),
                    extensions: Default::default(),
                    extras: Default::default(),
                    indices: Some(indices),
                    material: {
                        if mtl_id < num_materials as u32 {
                            Some(json::Index::new(mtl_id))
                        } else {
                            log!(msgs; "Material ID was found but no materials were specified.");
                            None
                        }
                    },
                    mode,
                    targets: targets.clone(),
                })
                .collect()
        } else {
            // None
            // Should have only one entry in indices if there are no materials detected: all faces have the same material.
            assert_eq!(indices.len(), 1);
            indices
                .into_iter()
                .map(|indices| {
                    json::mesh::Primitive {
                        attributes: build_attributes(),
                        extensions: Default::default(),
                        extras: Default::default(),
                        indices: Some(indices),
                        material: {
                            // Assign the material index only if there are materials there to prevent producing
                            // an invalid gltf.
                            if num_materials > 0 {
                                Some(json::Index::new(0))
                            } else {
                                None
                            }
                        },
                        mode,
                        targets: targets.clone(),
                    }
                })
                .collect()
        }
    } else {
        vec![json::mesh::Primitive {
            attributes: build_attributes(),
            extensions: Default::default(),
            extras: Default::default(),
            indices: None,
            material: {
                // Assign the material index only if there are materials there to prevent producing
                // an invalid gltf.
                if let Some(MaterialIds::Global { map }) = &attrib_transfer.material_ids {
                    let mtl_id = map.keys().next().unwrap_or(&0);
                    if *mtl_id < num_materials as u32 {
                        Some(json::Index::new(*mtl_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            mode,
            targets,
        }]
    }
}
