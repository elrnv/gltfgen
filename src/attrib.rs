use gltf::json;
use gut::mesh::topology::{FaceIndex, FaceVertexIndex, VertexIndex};
use gut::mesh::TriMesh;

pub(crate) type VertexAttribute = gut::mesh::attrib::Attribute<VertexIndex>;
pub(crate) type FaceVertexAttribute = gut::mesh::attrib::Attribute<FaceVertexIndex>;

pub(crate) type TexAttribDict = Vec<(TextureAttributeInfo, FaceVertexAttribute)>;
pub(crate) type AttribDict = Vec<(AttributeInfo, VertexAttribute)>;
pub(crate) type AttribTransfer = (AttribDict, TexAttribDict, /*material id*/ u32);

/// Cleanup unwanted attributes.
pub(crate) fn clean_attributes(
    mesh: &mut TriMesh<f32>,
    attributes: &[AttributeInfo],
    tex_attributes: &[TextureAttributeInfo],
    material_attribute: &String,
) -> AttribTransfer {
    // First we remove all attributes we want to keep.
    let attribs_to_keep: Vec<_> = attributes
        .iter()
        .filter_map(|attrib| remove_attribute(mesh, attrib))
        .collect();
    let tex_attribs_to_keep: Vec<_> = tex_attributes
        .iter()
        .filter_map(|attrib| remove_texture_coordinate_attribute(mesh, attrib))
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
fn remove_attribute(
    mesh: &mut TriMesh<f32>,
    attrib: &AttributeInfo,
) -> Option<(AttributeInfo, VertexAttribute)> {
    use gut::mesh::attrib::Attrib;
    mesh.remove_attrib::<VertexIndex>(&attrib.name)
        .ok()
        .map(|a| (attrib.clone(), a))
}

fn remove_texture_coordinate_attribute(
    mesh: &mut TriMesh<f32>,
    attrib: &TextureAttributeInfo,
) -> Option<(TextureAttributeInfo, FaceVertexAttribute)> {
    use gut::mesh::attrib::Attrib;
    mesh.remove_attrib::<FaceVertexIndex>(&attrib.name)
        .ok()
        .map(|a| (attrib.clone(), a))
}

#[macro_export]
macro_rules! call_typed_fn {
    ($attrib:expr => $prefix:ident :: $fn:ident :: <_$(,$params:ident)*> $args:tt ) => {
        {
            use json::accessor::{Type, ComponentType};
            match $attrib.type_ {
                Type::Scalar => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<i8 $(,$params)*> $args,
                    ComponentType::U8 =>  $prefix :: $fn::<u8 $(,$params)*> $args,
                    ComponentType::I16 => $prefix :: $fn::<i16 $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<u16 $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<u32 $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<f32 $(,$params)*>$args,
                },
                Type::Vec2 => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<[i8 ; 2] $(,$params)*>$args,
                    ComponentType::U8 =>  $prefix :: $fn::<[u8 ; 2] $(,$params)*>$args,
                    ComponentType::I16 => $prefix :: $fn::<[i16; 2] $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<[u16; 2] $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<[u32; 2] $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<[f32; 2] $(,$params)*>$args,
                },
                Type::Vec3 => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<[i8 ; 3] $(,$params)*>$args,
                    ComponentType::U8 =>  $prefix :: $fn::<[u8 ; 3] $(,$params)*>$args,
                    ComponentType::I16 => $prefix :: $fn::<[i16; 3] $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<[u16; 3] $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<[u32; 3] $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<[f32; 3] $(,$params)*>$args,
                },
                Type::Vec4 => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<[i8 ; 4] $(,$params)*>$args,
                    ComponentType::U8 =>  $prefix :: $fn::<[u8 ; 4] $(,$params)*>$args,
                    ComponentType::I16 => $prefix :: $fn::<[i16; 4] $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<[u16; 4] $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<[u32; 4] $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<[f32; 4] $(,$params)*>$args,
                },
                Type::Mat2 => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<[[i8 ; 2]; 2] $(,$params)*>$args,
                    ComponentType::U8 =>  $prefix :: $fn::<[[u8 ; 2]; 2] $(,$params)*>$args,
                    ComponentType::I16 => $prefix :: $fn::<[[i16; 2]; 2] $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<[[u16; 2]; 2] $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<[[u32; 2]; 2] $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<[[f32; 2]; 2] $(,$params)*>$args,
                },
                Type::Mat3 => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<[[i8 ; 3]; 3] $(,$params)*>$args,
                    ComponentType::U8 =>  $prefix :: $fn::<[[u8 ; 3]; 3] $(,$params)*>$args,
                    ComponentType::I16 => $prefix :: $fn::<[[i16; 3]; 3] $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<[[u16; 3]; 3] $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<[[u32; 3]; 3] $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<[[f32; 3]; 3] $(,$params)*>$args,
                },
                Type::Mat4 => match $attrib.component_type {
                    ComponentType::I8 =>  $prefix :: $fn::<[[i8 ; 4]; 4] $(,$params)*>$args,
                    ComponentType::U8 =>  $prefix :: $fn::<[[u8 ; 4]; 4] $(,$params)*>$args,
                    ComponentType::I16 => $prefix :: $fn::<[[i16; 4]; 4] $(,$params)*>$args,
                    ComponentType::U16 => $prefix :: $fn::<[[u16; 4]; 4] $(,$params)*>$args,
                    ComponentType::U32 => $prefix :: $fn::<[[u32; 4]; 4] $(,$params)*>$args,
                    ComponentType::F32 => $prefix :: $fn::<[[f32; 4]; 4] $(,$params)*>$args,
                },
            }
        }
    };
}

/*
 * Parsing attributes from command line
 */

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextureAttributeInfo {
    pub id: u32,
    pub name: String,
    pub component_type: json::accessor::ComponentType,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AttributeInfo {
    pub name: String,
    pub type_: json::accessor::Type,
    pub component_type: json::accessor::ComponentType,
}

fn parse_type(ty: &syn::Ident) -> Result<json::accessor::Type, syn::Error> {
    use json::accessor::Type;
    match ty.to_string().to_lowercase().as_str() {
        "scalar" => Ok(Type::Scalar),
        "vec2" => Ok(Type::Vec2),
        "vec3" => Ok(Type::Vec3),
        "vec4" => Ok(Type::Vec4),
        "mat2" => Ok(Type::Mat2),
        "mat3" => Ok(Type::Mat3),
        "mat4" => Ok(Type::Mat4),
        _ => Err(syn::Error::new(
            ty.span(),
            format!("invalid type: \"{}\". please choose one of Scalar, Vec2, Vec3, Vec4, Mat2, Mat3, or Mat4", ty),
        )),
    }
}

fn parse_component_type(ty: &syn::Ident) -> Result<json::accessor::ComponentType, syn::Error> {
    use json::accessor::ComponentType;
    match ty.to_string().to_lowercase().as_str() {
        "i8" => Ok(ComponentType::I8),
        "u8" => Ok(ComponentType::U8),
        "i16" => Ok(ComponentType::I16),
        "u16" => Ok(ComponentType::U16),
        "u32" => Ok(ComponentType::U32),
        "f32" => Ok(ComponentType::F32),
        _ => Err(syn::Error::new(
            ty.span(),
            format!(
                "invalid component type: \"{}\". Please choose one of i8, u8, i16, u16, u32 or f32",
                ty
            ),
        )),
    }
}

impl std::str::FromStr for AttributeInfo {
    type Err = syn::Error;
    fn from_str(input: &str) -> Result<AttributeInfo, Self::Err> {
        // Use syn to parse a type pattern and deconstruct it into something we can recognize.
        // NOTE: using syn is obviously overkill, but it is already available through dependencies,
        //       so it avoids an additional external dependency or writing a custom parser.
        syn::parse_str::<syn::ExprType>(input).and_then(|expr_type| {
            if let syn::Expr::Path(path) = *expr_type.expr {
                if let Some(name) = path.path.get_ident().map(|x| x.to_string()) {
                    if let syn::Type::Path(syn::TypePath { path, .. }) = *expr_type.ty {
                        if path.segments.len() != 1 {
                            return Err(syn::Error::new_spanned(
                                path,
                                "invalid format for attribute type",
                            ));
                        }

                        // We know we have exactly one path segment.
                        let ty = path.segments.first().unwrap();

                        // Try to first parse the type as a component type in case the user
                        // specifies the attribute as "attribute: f32". This is unambiguous so we
                        // accept it.
                        if let Ok(component_type) = parse_component_type(&ty.ident) {
                            return Ok(AttributeInfo {
                                name,
                                type_: json::accessor::Type::Scalar,
                                component_type,
                            });
                        }
                        parse_type(&ty.ident).and_then(|type_| {
                            if let syn::PathArguments::AngleBracketed(args) = &ty.arguments {
                                if args.args.len() != 1 {
                                    return Err(syn::Error::new_spanned(
                                        args,
                                        "invalid format for component type",
                                    ));
                                }

                                // We know there is exactly one component type arg.
                                if let syn::GenericArgument::Type(syn::Type::Path(
                                    syn::TypePath {
                                        path: component_type,
                                        ..
                                    },
                                )) = args.args.first().unwrap()
                                {
                                    component_type
                                        .get_ident()
                                        .ok_or_else(|| {
                                            syn::Error::new_spanned(
                                                component_type,
                                                "invalid format for component type",
                                            )
                                        })
                                        .and_then(|component_type| {
                                            parse_component_type(component_type).map(
                                                |component_type| AttributeInfo {
                                                    name,
                                                    type_,
                                                    component_type,
                                                },
                                            )
                                        })
                                } else {
                                    Err(syn::Error::new_spanned(
                                        ty,
                                        "invalid format for component type",
                                    ))
                                }
                            } else {
                                Err(syn::Error::new_spanned(
                                    ty,
                                    "invalid format for component type",
                                ))
                            }
                        })
                    } else {
                        Err(syn::Error::new_spanned(
                            expr_type.ty,
                            "invalid format for type",
                        ))
                    }
                } else {
                    Err(syn::Error::new_spanned(path, "invalid attribute name"))
                }
            } else {
                Err(syn::Error::new_spanned(expr_type, "invalid format"))
            }
        })
    }
}

impl std::str::FromStr for TextureAttributeInfo {
    type Err = syn::Error;
    fn from_str(input: &str) -> Result<TextureAttributeInfo, Self::Err> {
        // Use syn to parse a type pattern and deconstruct it into something we can recognize.
        // NOTE: using syn is obviously overkill, but it is already available through dependencies,
        //       so it avoids an additional external dependency or writing a custom parser.
        syn::parse_str::<syn::ExprType>(input).and_then(|expr_type| {
            if let syn::Expr::Path(path) = *expr_type.expr {
                if let Some(name) = path.path.get_ident().map(|x| x.to_string()) {
                    if let syn::Type::Path(syn::TypePath { path, .. }) = *expr_type.ty {
                        if path.segments.len() != 1 {
                            return Err(syn::Error::new_spanned(
                                path,
                                "invalid format for texture coordinate attribute type",
                            ));
                        }

                        // We know we have exactly one path segment.
                        let ty = path.segments.first().unwrap();

                        parse_component_type(&ty.ident).map(|component_type| TextureAttributeInfo {
                            id: 0,
                            name,
                            component_type,
                        })
                    } else {
                        Err(syn::Error::new_spanned(
                            expr_type.ty,
                            "invalid format for component type",
                        ))
                    }
                } else {
                    Err(syn::Error::new_spanned(
                        path,
                        "invalid texture coordinate attribute name",
                    ))
                }
            } else {
                Err(syn::Error::new_spanned(expr_type, "invalid format"))
            }
        })
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
