use serde::Deserialize;

/*
 * Parsing material info from command line
 */

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct TextureRef {
    pub index: u32,
    pub texcoord: u32,
}

fn default_base_color() -> [f32; 4] {
    [0.5, 0.5, 0.5, 1.0]
}

fn default_metallic() -> f32 {
    0.5
}

fn default_roughness() -> f32 {
    0.5
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MaterialInfo {
    pub name: Option<String>,
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 4],
    pub base_texture: Option<TextureRef>,
    #[serde(default = "default_metallic")]
    pub metallic: f32,
    #[serde(default = "default_roughness")]
    pub roughness: f32,
}

impl std::str::FromStr for MaterialInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<MaterialInfo, Self::Err> {
        ron::de::from_str::<MaterialInfo>(input)
    }
}
