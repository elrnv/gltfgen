use crate::mesh::Mesh;
use gltf::json;
use indexmap::map::IndexMap;
use meshx::mesh::TriMesh;
use meshx::topology::{FaceIndex, VertexIndex};
use serde::Deserialize;

type MaterialMap = IndexMap<meshx::io::obj::Material, Vec<usize>>;
type MaterialIdMap = IndexMap<u32, Vec<usize>>;

#[derive(Debug)]
pub enum AttribError {
    InvalidTexCoordAttribType(ComponentType),
    Mesh(meshx::attrib::Error),
}

impl std::error::Error for AttribError {}

impl std::fmt::Display for AttribError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttribError::InvalidTexCoordAttribType(t) => write!(
                f,
                "Invalid texture coordinate attribute type detected: {:?}. Skipping...",
                t
            ),
            AttribError::Mesh(e) => write!(f, "Mesh: {}", e),
        }
    }
}

impl From<meshx::attrib::Error> for AttribError {
    fn from(e: meshx::attrib::Error) -> AttribError {
        AttribError::Mesh(e)
    }
}

pub type VertexAttribute = meshx::attrib::Attribute<VertexIndex>;

#[derive(Clone, Debug, PartialEq)]
pub enum MaterialIds {
    /// Local material IDs inferred if materials are explicitly specified in the input mesh.
    Local {
        /// A map of materials with an associated list of face indices which use this material.
        map: MaterialMap,
    },
    /// Material IDs explicitly specified in the input mesh, which are interpreted as indexing some global material array.
    Global { map: MaterialIdMap },
}

#[derive(Clone, Debug)]
pub struct AttribTransfer {
    pub attribs_to_keep: Vec<Attribute>,
    pub color_attribs_to_keep: Vec<Attribute>,
    pub tex_attribs_to_keep: Vec<TextureAttribute>,
    pub material_ids: Option<MaterialIds>,
}

/// Find per face material IDs in the given mesh by probing a given integer type `I`.
fn find_material_ids<I: Clone + num_traits::ToPrimitive + 'static>(
    mesh: &Mesh,
    attrib_name: &str,
) -> Option<Vec<u32>> {
    use meshx::attrib::Attrib;
    match mesh {
        Mesh::TriMesh(mesh) => mesh
            .attrib_iter::<I, FaceIndex>(attrib_name)
            .ok()
            .map(|iter| {
                iter.map(|x| {
                    x.to_u32()
                        .expect("Material ID does not fit into an unsigned 32 bit integer.")
                })
                .collect()
            }),
        Mesh::PointCloud(ptcloud) => {
            ptcloud
                .attrib_iter::<I, VertexIndex>(attrib_name)
                .ok()
                .map(|iter| {
                    iter.map(|x| {
                        x.to_u32()
                            .expect("Material ID does not fit into an unsigned 32 bit integer.")
                    })
                    .collect()
                })
        }
    }
}

/// Cleanup unwanted attributes from a given `Mesh`.
pub(crate) fn clean_attributes(
    mesh: &mut Mesh,
    attributes: &AttributeInfo,
    color_attribs: &AttributeInfo,
    tex_attributes: &TextureAttributeInfo,
    material_attribute: &str,
    mut process_attrib_error: impl FnMut(AttribError),
) -> AttribTransfer {
    // First we remove all attributes we want to keep.
    let tex_attribs_to_keep: Vec<_> = if let Mesh::TriMesh(mesh) = mesh {
        tex_attributes
            .0
            .iter()
            .enumerate()
            .filter_map(|(id, attrib)| {
                match promote_and_remove_texture_coordinate_attribute(mesh, attrib, id) {
                    Err(e) => {
                        process_attrib_error(e);
                        None
                    }
                    Ok(r) => Some(r),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // It is important that these follow the tex attrib function since that can change mesh
    // topology.
    let attribs_to_keep: Vec<_> = attributes
        .0
        .iter()
        .filter_map(|attrib| remove_attribute(mesh, attrib))
        .collect();
    let color_attribs_to_keep: Vec<_> = color_attribs
        .0
        .iter()
        .filter_map(|attrib| remove_attribute(mesh, attrib))
        .collect();

    // Find material indices in this mesh.
    // Try a bunch of different integer types or look for a material attribute found in wavefront-obj imports.
    let material_ids = find_material_ids::<u32>(mesh, material_attribute)
        .or_else(|| find_material_ids::<i32>(mesh, material_attribute))
        .or_else(|| find_material_ids::<i64>(mesh, material_attribute))
        .or_else(|| find_material_ids::<u64>(mesh, material_attribute))
        .or_else(|| find_material_ids::<i16>(mesh, material_attribute))
        .or_else(|| find_material_ids::<u16>(mesh, material_attribute))
        .or_else(|| find_material_ids::<i8>(mesh, material_attribute))
        .or_else(|| find_material_ids::<u8>(mesh, material_attribute))
        .map_or_else(
            || {
                // Find the "mtl" attribute loaded by meshx for obj files.
                //
                // This allows gltfgen to automatically determine materials from
                // .mtl files without having to specify materials in the
                // configuration.
                let map = extract_mtls(mesh);
                if map.is_empty() {
                    None
                } else {
                    Some(MaterialIds::Local { map })
                }
            },
            |ids| {
                Some(MaterialIds::Global {
                    map: group_mtls(ids.as_slice()),
                })
            },
        );

    // Remove all attributes from the mesh.
    // It is important to delete these attributes, because they could cause a huge memory overhead.
    match mesh {
        Mesh::TriMesh(mesh) => {
            mesh.vertex_attributes.clear();
            mesh.face_attributes.clear();
            mesh.face_vertex_attributes.clear();
            mesh.face_edge_attributes.clear();
        }
        Mesh::PointCloud(ptcloud) => {
            ptcloud.vertex_attributes.clear();
        }
    }

    // Instead of reinserting back into the mesh, we keep this outside the mesh so we can
    // determine the type of the attribute.
    AttribTransfer {
        attribs_to_keep,
        color_attribs_to_keep,
        tex_attribs_to_keep,
        material_ids,
    }
}

/// Find and extract the "mtl" attribute loaded by `meshx` for obj files. This allows `gltfgen` to automatically determine
/// what textures to load.
fn extract_mtls(mesh: &mut Mesh) -> MaterialMap {
    use meshx::attrib::Attrib;
    let mut mtls = IndexMap::new();
    if let Ok(attrib) = match mesh {
        Mesh::TriMesh(mesh) => mesh.remove_attrib::<FaceIndex>("mtl"),
        Mesh::PointCloud(_) => return mtls, // Automatic materials not supported on pointclouds
    } {
        for (face_idx, mtl) in attrib
            .indirect_iter::<meshx::io::obj::Material>()
            .unwrap()
            .enumerate()
        {
            let face_indices: &mut Vec<usize> = mtls.entry(mtl.clone()).or_insert_with(Vec::new);
            face_indices.push(face_idx);
        }
    }
    mtls
}

/// Group the given list of material ids into groups of indices corresponding to the same id.
fn group_mtls(ids: &[u32]) -> MaterialIdMap {
    let mut map = IndexMap::new();
    for (face_idx, &mtl_id) in ids.iter().enumerate() {
        let face_indices: &mut Vec<usize> = map.entry(mtl_id).or_insert_with(Vec::new);
        face_indices.push(face_idx);
    }
    map
}

/// Remove the given attribute from the mesh and return it along with its name.
fn remove_attribute(mesh: &mut Mesh, attrib: (&String, &Type)) -> Option<Attribute> {
    use meshx::attrib::Attrib;
    match mesh {
        Mesh::TriMesh(mesh) => mesh.remove_attrib::<VertexIndex>(attrib.0),
        Mesh::PointCloud(mesh) => mesh.remove_attrib::<VertexIndex>(attrib.0),
    }
    .ok()
    .map(|a| Attribute {
        name: attrib.0.clone(),
        type_: *attrib.1,
        attribute: a,
    })
}

/// Try to promote the texture coordinate attribute from `FaceVertex` attribute to `Vertex`
/// attribute.
fn try_tex_coord_promote<T>(name: &str, mesh: &mut TriMesh<f32>) -> Result<(), AttribError>
where
    T: PartialEq + Clone + std::fmt::Debug + 'static,
{
    use meshx::attrib::AttribPromote;
    let err = "Texture coordinate collisions detected. Please report this issue.";
    Ok(mesh
        .attrib_promote::<[T; 2], _>(name, |a, b| assert_eq!(&*a, b, "{}", err))
        .map(|_| ())
        .or_else(|_| {
            mesh.attrib_promote::<[T; 3], _>(name, |a, b| assert_eq!(&*a, b, "{}", err))
                .map(|_| ())
        })?)
}

/// Promote the given attribute to from a face-vertex to a vertex attribute.
///
/// This is done by splitting the vertex positions for
/// unique values of the given face-vertex attribute. Then remove this attribute from the mesh for
/// later transfer.
///
/// If the given texture attribute is already a vertex attribute, skip the promotion stage.
fn promote_and_remove_texture_coordinate_attribute(
    mesh: &mut TriMesh<f32>,
    attrib: (&String, &ComponentType),
    id: usize,
) -> Result<TextureAttribute, AttribError> {
    use meshx::attrib::Attrib;
    use meshx::topology::FaceVertexIndex;

    if mesh.attrib_exists::<FaceVertexIndex>(attrib.0) {
        // Split the mesh according to texture attributes such that every unique texture attribute
        // value will have its own unique vertex. This is required since gltf doesn't support multiple
        // topologies.

        mesh.split_vertices_by_face_vertex_attrib(attrib.0);

        match *attrib.1 {
            ComponentType::U8 => try_tex_coord_promote::<u8>(attrib.0, mesh),
            ComponentType::U16 => try_tex_coord_promote::<u16>(attrib.0, mesh),
            ComponentType::F32 => try_tex_coord_promote::<f32>(attrib.0, mesh),
            t => Err(AttribError::InvalidTexCoordAttribType(t)),
        }?;
    }

    // The attribute has been promoted, remove it from the mesh for later use.
    Ok(mesh
        .remove_attrib::<VertexIndex>(attrib.0)
        .map(|a| TextureAttribute {
            id: id as u32,
            name: attrib.0.clone(),
            component_type: *attrib.1,
            attribute: a,
        })?)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureAttribute {
    pub id: u32,
    pub name: String,
    pub component_type: ComponentType,
    pub attribute: VertexAttribute,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
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
pub enum ComponentType {
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

impl From<ComponentType> for json::accessor::ComponentType {
    fn from(t: ComponentType) -> json::accessor::ComponentType {
        match t {
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
pub enum Type {
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

impl From<Type> for (json::accessor::Type, json::accessor::ComponentType) {
    fn from(t: Type) -> (json::accessor::Type, json::accessor::ComponentType) {
        let type_ = match t {
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

        let component_type = match t {
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
pub struct TextureAttributeInfo(pub IndexMap<String, ComponentType>);

impl Default for TextureAttributeInfo {
    fn default() -> Self {
        TextureAttributeInfo(IndexMap::new())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AttributeInfo(pub IndexMap<String, Type>);

impl Default for AttributeInfo {
    fn default() -> Self {
        AttributeInfo(IndexMap::new())
    }
}

impl std::str::FromStr for AttributeInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<AttributeInfo, Self::Err> {
        let idx_map: Result<IndexMap<String, Type>, Self::Err> =
            ron::de::from_str(input).map_err(Self::Err::from);
        idx_map.map(AttributeInfo)
    }
}

impl std::str::FromStr for TextureAttributeInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<TextureAttributeInfo, Self::Err> {
        let idx_map: Result<IndexMap<String, ComponentType>, Self::Err> =
            ron::de::from_str(input).map_err(Self::Err::from);
        idx_map.map(TextureAttributeInfo)
    }
}
