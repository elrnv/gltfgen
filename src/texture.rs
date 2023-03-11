use gltf::json;
use json::validation::Checked;
use serde::Deserialize;

/*
 * Parsing textures from command line
 * The following structs are designed to reduce verbosity on command line.
 */

/// Magnification filter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MagFilter {
    /// Corresponds to `GL_NEAREST`.
    #[serde(alias = "nearest")]
    Nearest,
    /// Corresponds to `GL_LINEAR`.
    #[serde(alias = "linear")]
    Linear,
    None,
}

impl From<MagFilter> for Option<Checked<json::texture::MagFilter>> {
    fn from(mf: MagFilter) -> Option<Checked<json::texture::MagFilter>> {
        match mf {
            MagFilter::Nearest => Some(Checked::Valid(json::texture::MagFilter::Nearest)),
            MagFilter::Linear => Some(Checked::Valid(json::texture::MagFilter::Linear)),
            MagFilter::None => None,
        }
    }
}

impl Default for MagFilter {
    fn default() -> Self {
        MagFilter::None
    }
}

/// Minification filter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MinFilter {
    /// Corresponds to `GL_NEAREST`.
    #[serde(alias = "nearest")]
    Nearest,
    /// Corresponds to `GL_LINEAR`.
    #[serde(alias = "linear")]
    Linear,
    /// Corresponds to `GL_NEAREST_MIPMAP_NEAREST`.
    #[serde(alias = "nearest_mipmap_nearest")]
    NearestMipmapNearest,
    /// Corresponds to `GL_LINEAR_MIPMAP_NEAREST`.
    #[serde(alias = "linear_mipmap_nearest")]
    LinearMipmapNearest,
    /// Corresponds to `GL_NEAREST_MIPMAP_LINEAR`.
    #[serde(alias = "nearest_mipmap_linear")]
    NearestMipmapLinear,
    /// Corresponds to `GL_LINEAR_MIPMAP_LINEAR`.
    #[serde(alias = "linear_mipmap_linear")]
    LinearMipmapLinear,
    None,
}

impl From<MinFilter> for Option<Checked<json::texture::MinFilter>> {
    fn from(mf: MinFilter) -> Option<Checked<json::texture::MinFilter>> {
        match mf {
            MinFilter::Nearest => Some(Checked::Valid(json::texture::MinFilter::Nearest)),
            MinFilter::Linear => Some(Checked::Valid(json::texture::MinFilter::Linear)),
            MinFilter::NearestMipmapNearest => Some(Checked::Valid(
                json::texture::MinFilter::NearestMipmapNearest,
            )),
            MinFilter::LinearMipmapNearest => Some(Checked::Valid(
                json::texture::MinFilter::LinearMipmapNearest,
            )),
            MinFilter::NearestMipmapLinear => Some(Checked::Valid(
                json::texture::MinFilter::NearestMipmapLinear,
            )),
            MinFilter::LinearMipmapLinear => {
                Some(Checked::Valid(json::texture::MinFilter::LinearMipmapLinear))
            }
            MinFilter::None => None,
        }
    }
}

impl Default for MinFilter {
    fn default() -> Self {
        MinFilter::None
    }
}

/// Texture co-ordinate wrapping mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum WrappingMode {
    /// Corresponds to `GL_CLAMP_TO_EDGE`.
    #[serde(alias = "clamp_to_edge")]
    ClampToEdge,
    /// Corresponds to `GL_MIRRORED_REPEAT`.
    #[serde(alias = "mirrored_repeat")]
    MirroredRepeat,
    /// Corresponds to `GL_REPEAT`.
    #[serde(alias = "repeat")]
    Repeat,
}

impl From<WrappingMode> for Checked<json::texture::WrappingMode> {
    fn from(wm: WrappingMode) -> Checked<json::texture::WrappingMode> {
        match wm {
            WrappingMode::ClampToEdge => Checked::Valid(json::texture::WrappingMode::ClampToEdge),
            WrappingMode::MirroredRepeat => {
                Checked::Valid(json::texture::WrappingMode::MirroredRepeat)
            }
            WrappingMode::Repeat => Checked::Valid(json::texture::WrappingMode::Repeat),
        }
    }
}

impl Default for WrappingMode {
    fn default() -> Self {
        WrappingMode::Repeat
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TextureInfo {
    pub image: ImageInfo,
    #[serde(default)]
    pub wrap_s: WrappingMode,
    #[serde(default)]
    pub wrap_t: WrappingMode,
    #[serde(default)]
    pub mag_filter: MagFilter,
    #[serde(default)]
    pub min_filter: MinFilter,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum ImageInfo {
    Uri(String),
    Embed(String),
}

impl std::str::FromStr for TextureInfo {
    type Err = ron::de::Error;
    fn from_str(input: &str) -> Result<TextureInfo, Self::Err> {
        ron::de::from_str::<TextureInfo>(input).map_err(Self::Err::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_texture() {
        let tex = TextureInfo {
            image: ImageInfo::Uri("t.jpg".to_string()),
            wrap_s: WrappingMode::Repeat,
            wrap_t: WrappingMode::Repeat,
            mag_filter: MagFilter::Nearest,
            min_filter: MinFilter::None,
        };
        let expected: TextureInfo = ron::de::from_str(
            "(image:Uri(\"t.jpg\"),wrap_s:Repeat,wrap_t:Repeat,mag_filter:Nearest,)",
        )
        .unwrap();
        assert_eq!(expected, tex);
    }
}
