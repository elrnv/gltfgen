use crate::config::TIME_ATTRIB_NAME;
use crate::config::WEIGHTS_ATTRIB_NAME;
use crate::config::{
    NORMAL_DISPLACEMENT_ATTRIB_NAME, POSITION_DISPLACEMENT_ATTRIB_NAME,
    TANGENT_DISPLACEMENT_ATTRIB_NAME,
};

use super::build_buffer_vec3;
use super::builders::*;
use super::Morph;
use byteorder::{WriteBytesExt, LE};
use gltf::json;
use indicatif::ProgressBar;
use json::accessor::ComponentType as GltfComponentType;
use json::validation::Checked::Valid;
use std::mem;

pub(crate) fn build_morph_target(
    morph: &Morph,
    accessors: &mut Vec<json::Accessor>,
    buffer_views: &mut Vec<json::buffer::View>,
    data: &mut Vec<u8>,
) -> json::mesh::MorphTarget {
    let disp_acc_index = build_buffer_vec3(
        &morph.position_disp,
        accessors,
        buffer_views,
        data,
        POSITION_DISPLACEMENT_ATTRIB_NAME,
    );
    let normal_disp_acc_index = build_buffer_vec3(
        &morph.normal_disp,
        accessors,
        buffer_views,
        data,
        NORMAL_DISPLACEMENT_ATTRIB_NAME,
    );
    let tangent_disp_acc_index = build_buffer_vec3(
        &morph.tangent_disp,
        accessors,
        buffer_views,
        data,
        TANGENT_DISPLACEMENT_ATTRIB_NAME,
    );

    json::mesh::MorphTarget {
        positions: disp_acc_index,
        normals: normal_disp_acc_index,
        tangents: tangent_disp_acc_index,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_animation(
    first_frame: u32,
    morphs: &[Morph],
    node_index: usize,
    accessors: &mut Vec<json::Accessor>,
    buffer_views: &mut Vec<json::buffer::View>,
    data: &mut Vec<u8>,
    time_step: f32,
    insert_vanishing_frames: bool,
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

    let mut first_morph = 0;
    if insert_vanishing_frames {
        // First frame is vanishing, second is the actual first frame of the animation.
        // We need to order the weights so the frames are in order.
        data.write_u32::<LE>(0u32).unwrap();
        first_morph = 1;
    }
    // Note: first frame is all zeros
    for i in first_morph..morphs.len() {
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
            .with_name(WEIGHTS_ATTRIB_NAME.to_string())
            .with_min_max(&[0.0][..], &[1.0][..])
            .with_sparse(morphs.len(), weight_indices_view_index, weight_view_index);

    let weights_acc_index = accessors.len() as u32;
    accessors.push(weights_acc);

    // Animation keyframe times
    let byte_length = num_animation_frames * mem::size_of::<f32>();
    let time_view = json::buffer::View::new(byte_length, data.len());

    let mut min_time = first_frame as f32 * time_step;
    let mut max_time = first_frame as f32 * time_step;
    if insert_vanishing_frames {
        data.write_f32::<LE>(morphs[0].frame as f32 * time_step)
            .unwrap();
    }
    data.write_f32::<LE>(first_frame as f32 * time_step)
        .unwrap();
    for Morph { frame, .. } in morphs.iter() {
        let time = *frame as f32 * time_step;
        min_time = min_time.min(time);
        max_time = max_time.max(time);
        if insert_vanishing_frames && frame == &morphs[0].frame {
            // Skip the first vanishing frame frame, since it was already inserted above.
            continue;
        }
        data.write_f32::<LE>(time).unwrap();
    }
    let time_view_index = buffer_views.len();
    buffer_views.push(time_view);

    let time_acc = json::Accessor::new(num_animation_frames, GltfComponentType::F32)
        .with_name(TIME_ATTRIB_NAME.to_string())
        .with_buffer_view(time_view_index)
        .with_min_max(&[min_time][..], &[max_time][..]);

    let time_acc_index = accessors.len() as u32;
    accessors.push(time_acc);

    for morph in morphs.iter() {
        pb.tick();
        targets.push(build_morph_target(morph, accessors, buffer_views, data));
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
