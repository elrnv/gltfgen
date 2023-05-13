use gltf::json;
use json::validation::Checked::Valid;
use serde::Deserialize;

/*
 * Parsing material info from command line
 */

/// Specifies the texture to be used by the material.
#[derive(Copy, Clone, Debug, PartialEq, Default, Deserialize)]
#[serde(untagged)]
pub enum TextureRef {
    Some {
        /// Specifies the 0-based index of the texture in a separate input vector storing `TextureInfo`s.
        index: u32,
        /// Specifies the index of the texture attribute specified in a separate input vector storing `TextureAttributeInfo`s.
        texcoord: u32,
    },
    /// Indicates that texture is not set.
    #[default]
    None,
}

impl TextureRef {
    fn into_option(self) -> Option<(u32, u32)> {
        self.into()
    }
}

impl From<TextureRef> for Option<(u32, u32)> {
    fn from(tr: TextureRef) -> Option<(u32, u32)> {
        match tr {
            TextureRef::Some { index, texcoord } => Some((index, texcoord)),
            TextureRef::None => None,
        }
    }
}

fn default_base_color() -> [f32; 4] {
    [0.5, 0.5, 0.5, 1.0]
}

fn default_metallic() -> f32 {
    0.0
}

fn default_roughness() -> f32 {
    0.5
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MaterialInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 4],
    #[serde(default)]
    pub base_texture: TextureRef,
    #[serde(default = "default_metallic")]
    pub metallic: f32,
    #[serde(default = "default_roughness")]
    pub roughness: f32,
}

impl Default for MaterialInfo {
    fn default() -> Self {
        MaterialInfo {
            name: "Default".to_owned(),
            base_color: default_base_color(),
            base_texture: TextureRef::None,
            metallic: default_metallic(),
            roughness: default_roughness(),
        }
    }
}

impl std::str::FromStr for MaterialInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<MaterialInfo, Self::Err> {
        ron::de::from_str::<MaterialInfo>(input).map_err(Self::Err::from)
    }
}

/// Convenience converter using Material information from an obj material.
///
/// This conversion ignore textures.
impl From<&meshx::io::obj::Material> for MaterialInfo {
    fn from(mtl: &meshx::io::obj::Material) -> Self {
        let kd = mtl
            .kd
            .map(|kd| [kd[0].into_inner(), kd[1].into_inner(), kd[2].into_inner()])
            .unwrap_or_else(|| {
                let c = default_base_color();
                [c[0], c[1], c[2]]
            });
        let d = mtl
            .d
            .map(meshx::io::obj::NotNan::into_inner)
            .unwrap_or_else(|| {
                mtl.tr
                    .map(|tr| 1.0 - tr.into_inner())
                    .unwrap_or_else(|| default_base_color()[3])
            });
        MaterialInfo {
            name: mtl.name.clone(),
            base_color: [kd[0], kd[1], kd[2], d],
            // TODO: See https://en.wikipedia.org/wiki/Wavefront_.obj_file#Physically-based_Rendering
            // metallic: mtl.Pm,
            // roughness: mtl.Pr,
            ..Default::default()
        }
    }
}

impl From<MaterialInfo> for json::Material {
    fn from(mi: MaterialInfo) -> json::Material {
        let MaterialInfo {
            name,
            base_color,
            base_texture,
            metallic,
            roughness,
        } = mi;

        json::Material {
            name: if name.is_empty() { None } else { Some(name) },
            alpha_cutoff: None,
            alpha_mode: Valid(json::material::AlphaMode::Opaque),
            double_sided: false,
            pbr_metallic_roughness: json::material::PbrMetallicRoughness {
                base_color_factor: json::material::PbrBaseColorFactor(base_color),
                base_color_texture: base_texture.into_option().map(|(index, texcoord)| {
                    json::texture::Info {
                        index: json::Index::new(index),
                        tex_coord: texcoord,
                        extensions: Default::default(),
                        extras: Default::default(),
                    }
                }),
                metallic_factor: json::material::StrengthFactor(metallic),
                roughness_factor: json::material::StrengthFactor(roughness),
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
    }
}
