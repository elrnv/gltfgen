use super::builders::*;
use byteorder::{WriteBytesExt, LE};
use gltf::json;
use meshx::{ops::*, bbox::BBox};
use indicatif::ProgressBar;
use json::accessor::ComponentType as GltfComponentType;
use json::accessor::Type as GltfType;
use json::validation::Checked::Valid;
use std::mem;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_animation(
    first_frame: usize,
    morphs: &[(usize, Vec<[f32; 3]>)],
    node_index: usize,
    accessors: &mut Vec<json::Accessor>,
    buffer_views: &mut Vec<json::buffer::View>,
    data: &mut Vec<u8>,
    time_step: f32,
    pb: &ProgressBar,
) -> Option<(
    json::animation::Channel,
    json::animation::Sampler,
    Vec<json::mesh::MorphTarget>,
)> {
    if morphs.is_empty() {
        return None;
    }

    let mut targets = Vec::new();

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

    // Output animation frames as weights
    let weight_view = json::buffer::View::new(morphs.len() * mem::size_of::<f32>(), data.len());

    let weight_view_index = buffer_views.len();
    buffer_views.push(weight_view);

    for _ in 0..morphs.len() {
        data.write_f32::<LE>(1.0).unwrap();
    }

    // Weights accessor for all frames
    let weights_acc =
        json::Accessor::new(num_animation_frames * morphs.len(), GltfComponentType::F32)
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

    let time_acc = json::Accessor::new(num_animation_frames, GltfComponentType::F32)
        .with_buffer_view(time_view_index)
        .with_min_max(&[min_time][..], &[max_time][..]);

    let time_acc_index = accessors.len() as u32;
    accessors.push(time_acc);

    for (_, displacements) in morphs.iter() {
        pb.tick();
        let byte_length = displacements.len() * mem::size_of::<[f32; 3]>();

        let disp_view = json::buffer::View::new(byte_length, data.len())
            .with_stride(mem::size_of::<[f32; 3]>())
            .with_target(json::buffer::Target::ArrayBuffer);
        let disp_view_index = buffer_views.len();
        buffer_views.push(disp_view);

        let mut bbox = BBox::empty();
        for disp in displacements.iter() {
            bbox.absorb(*disp);
            for &coord in disp.iter() {
                data.write_f32::<LE>(coord).unwrap();
            }
        }

        let disp_acc = json::Accessor::new(displacements.len(), GltfComponentType::F32)
            .with_buffer_view(disp_view_index)
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
            node: json::Index::new(node_index as u32),
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

    Some((channel, sampler, targets))
}
