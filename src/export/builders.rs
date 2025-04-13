//! This module includes convenience builders that are missing from gltf_json for some reason as
//! well as conveinent byte writers.
//!
//! Some of these may be obsolete when the gltf crate is updated.

use crate::attrib::*;
use gltf::json;
use json::accessor::ComponentType as GltfComponentType;
use json::accessor::Type as GltfType;

use byteorder::{WriteBytesExt, LE};
use json::validation::Checked::Valid;

pub(crate) trait BufferViewBuilder {
    fn new(byte_length: usize, byte_offset: usize) -> Self;
    fn with_target(self, target: json::buffer::Target) -> Self;
    fn with_stride(self, byte_stride: usize) -> Self;
}

impl BufferViewBuilder for json::buffer::View {
    fn new(byte_length: usize, byte_offset: usize) -> json::buffer::View {
        json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: byte_length.into(),
            byte_offset: Some(byte_offset.into()),
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
        self.byte_stride = Some(json::buffer::Stride(byte_stride));
        self
    }
}

pub trait AccessorBuilder {
    fn new(count: usize, generic_comp: GltfComponentType) -> Self;
    fn with_name(self, name: String) -> Self;
    fn with_buffer_view(self, buffer_view: usize) -> Self;
    #[allow(dead_code)]
    fn with_byte_offset(self, byte_offset: usize) -> Self;
    fn with_type(self, type_: GltfType) -> Self;
    #[allow(dead_code)]
    fn with_component_type(self, component_type: json::accessor::GenericComponentType) -> Self;
    fn with_min_max<'a, T>(self, min: &'a [T], max: &'a [T]) -> Self
    where
        json::Value: From<&'a [T]>;
    fn with_sparse(self, count: usize, indices_buf_view: usize, values_buf_view: usize) -> Self;
}

impl AccessorBuilder for json::Accessor {
    /// Assumes scalar type.
    fn new(count: usize, generic_component_type: GltfComponentType) -> json::Accessor {
        json::Accessor {
            buffer_view: None,
            byte_offset: None,
            count: count.into(),
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
    fn with_name(mut self, name: String) -> json::Accessor {
        self.name = Some(name);
        self
    }
    fn with_buffer_view(mut self, buf_view: usize) -> json::Accessor {
        if self.byte_offset.is_none() {
            self.byte_offset = Some(0_u64.into());
        }
        self.buffer_view = Some(json::Index::new(buf_view as u32));
        self
    }
    fn with_byte_offset(mut self, byte_offset: usize) -> json::Accessor {
        self.byte_offset = Some(byte_offset.into());
        self
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
            count: count.into(),
            indices: json::accessor::sparse::Indices {
                buffer_view: json::Index::new(indices_buf_view as u32),
                byte_offset: 0_u64.into(),
                component_type: Valid(json::accessor::IndexComponentType(GltfComponentType::U32)),
                extensions: Default::default(),
                extras: Default::default(),
            },
            values: json::accessor::sparse::Values {
                buffer_view: json::Index::new(values_buf_view as u32),
                byte_offset: 0_u64.into(),
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
pub(crate) trait WriteBytes {
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

pub(crate) fn write_attribute_data<T: WriteBytes + 'static>(
    data: &mut Vec<u8>,
    attrib: &Attribute,
) {
    let iter = VertexAttribute::iter::<T>(&attrib.attribute).unwrap_or_else(|_| {
        panic!(
            "Unsupported attribute: \"{:?}\"",
            (attrib.name.as_str(), attrib.type_)
        )
    });
    iter.for_each(|x| x.write_bytes(data));
}

pub(crate) fn write_tex_attribute_data<T: Copy + WriteBytes + 'static>(
    data: &mut Vec<u8>,
    attrib: &TextureAttribute,
) {
    if let Ok(iter) = VertexAttribute::iter::<[T; 2]>(&attrib.attribute) {
        iter.for_each(|x| x.write_bytes(data));
    } else if let Ok(iter) = VertexAttribute::iter::<[T; 3]>(&attrib.attribute) {
        iter.for_each(|&[a, b, _]| [a, b].write_bytes(data));
    // Be lenient and try a 3 vector. Sometime uv coordinates are stored in a 3D vector.
    } else {
        log::warn!(
            "Unsupported texture coordinate attribute: \"{:?}\". Skipping...",
            (attrib.name.as_str(), attrib.component_type)
        );
    }
}

pub(crate) fn write_color_attribute_data<T: Copy + WriteBytes + 'static>(
    data: &mut Vec<u8>,
    attrib: &Attribute,
) {
    if let Ok(iter) = VertexAttribute::iter::<T>(&attrib.attribute) {
        iter.for_each(|x| x.write_bytes(data));
    } else {
        log::warn!(
            "Unsupported color coordinate attribute: \"{:?}\". Skipping...",
            (attrib.name.as_str(), attrib.type_)
        );
    }
}
