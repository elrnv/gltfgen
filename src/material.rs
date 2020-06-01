use gltf::json;
use json::validation::Checked::Valid;
use serde::Deserialize;

/*
 * Parsing material info from command line
 */

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
pub struct TextureRef {
    pub index: u32,
    pub texcoord: u32,
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
    pub base_texture: Option<TextureRef>,
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
            base_texture: None,
            metallic: default_metallic(),
            roughness: default_roughness(),
        }
    }
}

impl std::str::FromStr for MaterialInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<MaterialInfo, Self::Err> {
        ron::de::from_str::<MaterialInfo>(input)
    }
}

impl Into<json::Material> for MaterialInfo {
    fn into(self) -> json::Material {
        let MaterialInfo {
            name,
            base_color,
            base_texture,
            metallic,
            roughness,
        } = self;

        json::Material {
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_owned())
            },
            alpha_cutoff: json::material::AlphaCutoff(0.5),
            alpha_mode: Valid(json::material::AlphaMode::Opaque),
            double_sided: false,
            pbr_metallic_roughness: json::material::PbrMetallicRoughness {
                base_color_factor: json::material::PbrBaseColorFactor(base_color),
                base_color_texture: base_texture.map(|base_texture| json::texture::Info {
                    index: json::Index::new(base_texture.index),
                    tex_coord: base_texture.texcoord,
                    extensions: Default::default(),
                    extras: Default::default(),
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
