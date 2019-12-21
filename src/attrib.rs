use gltf::json;
use gut::mesh::topology::{FaceIndex, FaceVertexIndex, VertexIndex};
use gut::mesh::TriMesh;
use indexmap::map::IndexMap;
use serde::Deserialize;

pub(crate) type VertexAttribute = gut::mesh::attrib::Attribute<VertexIndex>;
pub(crate) type FaceVertexAttribute = gut::mesh::attrib::Attribute<FaceVertexIndex>;

pub(crate) type AttribTransfer = (
    Vec<Attribute>,
    Vec<TextureAttribute>,
    /*material id*/ u32,
);

/// Cleanup unwanted attributes.
pub(crate) fn clean_attributes(
    mesh: &mut TriMesh<f32>,
    attributes: &AttributeInfo,
    tex_attributes: &TextureAttributeInfo,
    material_attribute: &String,
) -> AttribTransfer {
    // First we remove all attributes we want to keep.
    let attribs_to_keep: Vec<_> = attributes
        .0
        .iter()
        .filter_map(|attrib| remove_attribute(mesh, attrib))
        .collect();
    let tex_attribs_to_keep: Vec<_> = tex_attributes
        .0
        .iter()
        .enumerate()
        .filter_map(|(id, attrib)| remove_texture_coordinate_attribute(mesh, attrib, id))
        .collect();

    // Compute the material index for this mesh.
    use gut::mesh::attrib::Attrib;
    let material_id = mesh
        .attrib_iter::<u32, FaceIndex>(material_attribute)
        .map(|x| mode(x.cloned()).0)
        .unwrap_or(0);

    // Remove all attributes from the mesh.
    // It is important to delete these attributes, because they could cause a huge memory overhead.
    mesh.vertex_attributes.clear();
    mesh.face_attributes.clear();
    mesh.face_vertex_attributes.clear();
    mesh.face_edge_attributes.clear();

    // Instead of reinserting back into the mesh, we keep this outside the mesh so we can
    // determine the type of the attribute.
    (attribs_to_keep, tex_attribs_to_keep, material_id)
}

/// Remove the given attribute from the mesh and return it along with its name.
fn remove_attribute(mesh: &mut TriMesh<f32>, attrib: (&String, &Type)) -> Option<Attribute> {
    use gut::mesh::attrib::Attrib;
    mesh.remove_attrib::<VertexIndex>(&attrib.0)
        .ok()
        .map(|a| Attribute {
            name: attrib.0.clone(),
            type_: *attrib.1,
            attribute: a,
        })
}

fn remove_texture_coordinate_attribute(
    mesh: &mut TriMesh<f32>,
    attrib: (&String, &ComponentType),
    id: usize,
) -> Option<TextureAttribute> {
    use gut::mesh::attrib::Attrib;
    mesh.remove_attrib::<FaceVertexIndex>(&attrib.0)
        .ok()
        .map(|a| TextureAttribute {
            id: id as u32,
            name: attrib.0.clone(),
            component_type: *attrib.1,
            attribute: a,
        })
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextureAttribute {
    pub id: u32,
    pub name: String,
    pub component_type: ComponentType,
    pub attribute: FaceVertexAttribute,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Attribute {
    pub name: String,
    pub type_: Type,
    pub attribute: VertexAttribute,
}

#[macro_export]
macro_rules! call_typed_fn {
    ($type:expr => $prefix:ident :: $fn:ident :: <_$(,$params:ident)*> $args:tt ) => {
        {
            match $type {
                Type::Scalar(ComponentType::I8)  | Type::I8 =>  $prefix :: $fn::<i8 $(,$params)*> $args,
                Type::Scalar(ComponentType::U8)  | Type::U8 =>  $prefix :: $fn::<u8 $(,$params)*> $args,
                Type::Scalar(ComponentType::I16) | Type::I16 => $prefix :: $fn::<i16 $(,$params)*>$args,
                Type::Scalar(ComponentType::U16) | Type::U16 => $prefix :: $fn::<u16 $(,$params)*>$args,
                Type::Scalar(ComponentType::U32) | Type::U32 => $prefix :: $fn::<u32 $(,$params)*>$args,
                Type::Scalar(ComponentType::F32) | Type::F32 => $prefix :: $fn::<f32 $(,$params)*>$args,

                Type::Vec2(ComponentType::I8 ) => $prefix :: $fn::<[i8 ; 2] $(,$params)*>$args,
                Type::Vec2(ComponentType::U8 ) => $prefix :: $fn::<[u8 ; 2] $(,$params)*>$args,
                Type::Vec2(ComponentType::I16) => $prefix :: $fn::<[i16; 2] $(,$params)*>$args,
                Type::Vec2(ComponentType::U16) => $prefix :: $fn::<[u16; 2] $(,$params)*>$args,
                Type::Vec2(ComponentType::U32) => $prefix :: $fn::<[u32; 2] $(,$params)*>$args,
                Type::Vec2(ComponentType::F32) => $prefix :: $fn::<[f32; 2] $(,$params)*>$args,

                Type::Vec3(ComponentType::I8 ) => $prefix :: $fn::<[i8 ; 3] $(,$params)*>$args,
                Type::Vec3(ComponentType::U8 ) => $prefix :: $fn::<[u8 ; 3] $(,$params)*>$args,
                Type::Vec3(ComponentType::I16) => $prefix :: $fn::<[i16; 3] $(,$params)*>$args,
                Type::Vec3(ComponentType::U16) => $prefix :: $fn::<[u16; 3] $(,$params)*>$args,
                Type::Vec3(ComponentType::U32) => $prefix :: $fn::<[u32; 3] $(,$params)*>$args,
                Type::Vec3(ComponentType::F32) => $prefix :: $fn::<[f32; 3] $(,$params)*>$args,

                Type::Vec4(ComponentType::I8 ) => $prefix :: $fn::<[i8 ; 4] $(,$params)*>$args,
                Type::Vec4(ComponentType::U8 ) => $prefix :: $fn::<[u8 ; 4] $(,$params)*>$args,
                Type::Vec4(ComponentType::I16) => $prefix :: $fn::<[i16; 4] $(,$params)*>$args,
                Type::Vec4(ComponentType::U16) => $prefix :: $fn::<[u16; 4] $(,$params)*>$args,
                Type::Vec4(ComponentType::U32) => $prefix :: $fn::<[u32; 4] $(,$params)*>$args,
                Type::Vec4(ComponentType::F32) => $prefix :: $fn::<[f32; 4] $(,$params)*>$args,

                Type::Mat2(ComponentType::I8 ) =>  $prefix :: $fn::<[[i8 ; 2]; 2] $(,$params)*>$args,
                Type::Mat2(ComponentType::U8 ) =>  $prefix :: $fn::<[[u8 ; 2]; 2] $(,$params)*>$args,
                Type::Mat2(ComponentType::I16) => $prefix :: $fn::<[[i16; 2]; 2] $(,$params)*>$args,
                Type::Mat2(ComponentType::U16) => $prefix :: $fn::<[[u16; 2]; 2] $(,$params)*>$args,
                Type::Mat2(ComponentType::U32) => $prefix :: $fn::<[[u32; 2]; 2] $(,$params)*>$args,
                Type::Mat2(ComponentType::F32) => $prefix :: $fn::<[[f32; 2]; 2] $(,$params)*>$args,

                Type::Mat3(ComponentType::I8 ) => $prefix :: $fn::<[[i8 ; 3]; 3] $(,$params)*>$args,
                Type::Mat3(ComponentType::U8 ) => $prefix :: $fn::<[[u8 ; 3]; 3] $(,$params)*>$args,
                Type::Mat3(ComponentType::I16) => $prefix :: $fn::<[[i16; 3]; 3] $(,$params)*>$args,
                Type::Mat3(ComponentType::U16) => $prefix :: $fn::<[[u16; 3]; 3] $(,$params)*>$args,
                Type::Mat3(ComponentType::U32) => $prefix :: $fn::<[[u32; 3]; 3] $(,$params)*>$args,
                Type::Mat3(ComponentType::F32) => $prefix :: $fn::<[[f32; 3]; 3] $(,$params)*>$args,

                Type::Mat4(ComponentType::I8 ) => $prefix :: $fn::<[[i8 ; 4]; 4] $(,$params)*>$args,
                Type::Mat4(ComponentType::U8 ) => $prefix :: $fn::<[[u8 ; 4]; 4] $(,$params)*>$args,
                Type::Mat4(ComponentType::I16) => $prefix :: $fn::<[[i16; 4]; 4] $(,$params)*>$args,
                Type::Mat4(ComponentType::U16) => $prefix :: $fn::<[[u16; 4]; 4] $(,$params)*>$args,
                Type::Mat4(ComponentType::U32) => $prefix :: $fn::<[[u32; 4]; 4] $(,$params)*>$args,
                Type::Mat4(ComponentType::F32) => $prefix :: $fn::<[[f32; 4]; 4] $(,$params)*>$args,
            }
        }
    };
}

/*
 * Parsing attributes from command line
 */

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
pub(crate) enum ComponentType {
    /// Signed 8-bit integer. Corresponds to `GL_BYTE`.
    #[serde(alias = "i8")]
    I8,
    /// Unsigned 8-bit integer. Corresponds to `GL_UNSIGNED_BYTE`.
    #[serde(alias = "u8")]
    U8,
    /// Signed 16-bit integer. Corresponds to `GL_SHORT`.
    #[serde(alias = "i16")]
    I16,
    /// Unsigned 16-bit integer. Corresponds to `GL_UNSIGNED_SHORT`.
    #[serde(alias = "u16")]
    U16,
    /// Unsigned 32-bit integer. Corresponds to `GL_UNSIGNED_INT`.
    #[serde(alias = "u32")]
    U32,
    /// Single precision (32-bit) floating point number. Corresponds to `GL_FLOAT`.
    #[serde(alias = "f32")]
    F32,
}

impl Into<json::accessor::ComponentType> for ComponentType {
    fn into(self) -> json::accessor::ComponentType {
        match self {
            ComponentType::I8 => json::accessor::ComponentType::I8,
            ComponentType::U8 => json::accessor::ComponentType::U8,
            ComponentType::I16 => json::accessor::ComponentType::I16,
            ComponentType::U16 => json::accessor::ComponentType::U16,
            ComponentType::U32 => json::accessor::ComponentType::U32,
            ComponentType::F32 => json::accessor::ComponentType::F32,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
pub(crate) enum Type {
    /// Signed 8-bit integer. Corresponds to `GL_BYTE`.
    #[serde(alias = "i8")]
    I8,
    /// Unsigned 8-bit integer. Corresponds to `GL_UNSIGNED_BYTE`.
    #[serde(alias = "u8")]
    U8,
    /// Signed 16-bit integer. Corresponds to `GL_SHORT`.
    #[serde(alias = "i16")]
    I16,
    /// Unsigned 16-bit integer. Corresponds to `GL_UNSIGNED_SHORT`.
    #[serde(alias = "u16")]
    U16,
    /// Unsigned 32-bit integer. Corresponds to `GL_UNSIGNED_INT`.
    #[serde(alias = "u32")]
    U32,
    /// Single precision (32-bit) floating point number. Corresponds to `GL_FLOAT`.
    #[serde(alias = "f32")]
    F32,
    /// Scalar quantity.
    #[serde(alias = "scalar")]
    Scalar(ComponentType),
    /// 2D vector.
    #[serde(alias = "vec2")]
    Vec2(ComponentType),
    /// 3D vector.
    #[serde(alias = "vec3")]
    Vec3(ComponentType),
    /// 4D vector.
    #[serde(alias = "vec4")]
    Vec4(ComponentType),
    /// 2x2 matrix.
    #[serde(alias = "mat2")]
    Mat2(ComponentType),
    /// 3x3 matrix.
    #[serde(alias = "mat3")]
    Mat3(ComponentType),
    /// 4x4 matrix.
    #[serde(alias = "mat4")]
    Mat4(ComponentType),
}

impl Into<(json::accessor::Type, json::accessor::ComponentType)> for Type {
    fn into(self) -> (json::accessor::Type, json::accessor::ComponentType) {
        let type_ = match self {
            Type::I8
            | Type::U8
            | Type::I16
            | Type::U16
            | Type::U32
            | Type::F32
            | Type::Scalar(_) => json::accessor::Type::Scalar,
            Type::Vec2(_) => json::accessor::Type::Vec2,
            Type::Vec3(_) => json::accessor::Type::Vec3,
            Type::Vec4(_) => json::accessor::Type::Vec4,
            Type::Mat2(_) => json::accessor::Type::Mat2,
            Type::Mat3(_) => json::accessor::Type::Mat3,
            Type::Mat4(_) => json::accessor::Type::Mat4,
        };

        let component_type = match self {
            Type::I8 => json::accessor::ComponentType::I8,
            Type::U8 => json::accessor::ComponentType::U8,
            Type::I16 => json::accessor::ComponentType::I16,
            Type::U16 => json::accessor::ComponentType::U16,
            Type::U32 => json::accessor::ComponentType::U32,
            Type::F32 => json::accessor::ComponentType::F32,
            Type::Scalar(c)
            | Type::Vec2(c)
            | Type::Vec3(c)
            | Type::Vec4(c)
            | Type::Mat2(c)
            | Type::Mat3(c)
            | Type::Mat4(c) => c.into(),
        };

        (type_, component_type)
    }
}

// Note that indexmap is essential here since we want to preserve the order of the texture
// coordinate attributes since we are using it explicitly in the gltf output.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct TextureAttributeInfo(pub IndexMap<String, ComponentType>);

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AttributeInfo(pub IndexMap<String, Type>);

impl std::str::FromStr for AttributeInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<AttributeInfo, Self::Err> {
        let idx_map: Result<IndexMap<String, Type>, Self::Err> = ron::de::from_str(input);
        idx_map.map(|m| AttributeInfo(m))
    }
}

impl std::str::FromStr for TextureAttributeInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<TextureAttributeInfo, Self::Err> {
        let idx_map: Result<IndexMap<String, ComponentType>, Self::Err> = ron::de::from_str(input);
        idx_map.map(|m| TextureAttributeInfo(m))
    }
}

/// Given a slice of integers, compute the mode and return it along with its
/// frequency.
/// If the slice is empty just return 0.
fn mode<I: IntoIterator<Item = u32>>(data: I) -> (u32, usize) {
    let data_iter = data.into_iter();
    let mut bins = Vec::with_capacity(100);
    for x in data_iter {
        let i = x as usize;
        if i >= bins.len() {
            bins.resize(i + 1, 0usize);
        }
        bins[i] += 1;
    }
    bins.iter()
        .cloned()
        .enumerate()
        .max_by_key(|&(_, f)| f)
        .map(|(m, f)| (m as u32, f))
        .unwrap_or((0u32, 0))
}

#[test]
fn mode_test() {
    let v = vec![1u32, 1, 1, 0, 0, 0, 0, 1, 2, 2, 1, 0, 1];
    assert_eq!(mode(v), (1, 6));
    let v = vec![];
    assert_eq!(mode(v), (0, 0));
    let v = vec![0u32, 0, 0, 1, 1, 1, 1, 2, 2, 2];
    assert_eq!(mode(v), (1, 4));
}