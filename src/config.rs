use clap::Parser;
use serde::Deserialize;

use crate::{AttributeInfo, MaterialInfo, TextureAttributeInfo, TextureInfo};

fn default_fps() -> usize {
    24
}
fn default_step() -> usize {
    1
}
fn default_mtl_id() -> String {
    "mtl_id".to_string()
}
fn default_textures() -> Vec<TextureInfo> {
    vec![TextureInfo::default()]
}

/// Output configuration for the generated glTF.
#[derive(Parser, Debug, Deserialize)]
pub struct Config {
    /// Frames per second.
    ///
    /// 1/fps gives the time step between discrete frames. If 'time_step' is also provided, this
    /// parameter is ignored.
    #[clap(value_name = "FPS", short, long, default_value = "24")]
    #[serde(default = "default_fps")]
    pub fps: usize,

    /// Time step in seconds between discrete frames.
    ///
    /// Specifying this option overrides the time step that would be computed from 'fps', which is
    /// set to 24 by default.  This means that the default 'time_step' is equivalently 1/24.
    #[clap(value_name = "TIMESTEP", short, long)]
    pub time_step: Option<f32>,

    /// Reverse polygon orientations on output meshes.
    #[clap(short, long)]
    #[serde(default)]
    pub reverse: bool,

    /// Invert tetrahedra orientations on input meshes.
    #[clap(short, long)]
    #[serde(default)]
    pub invert_tets: bool,

    /// Step by the given number of frames.
    ///
    /// In other words, read frames in increments of 'step'.  Note that this
    /// does not affect the value for 'fps' or 'time_step' options.  This number
    /// must be at least 1.
    ///
    /// For example for frames 1 to 10, a 'step' value of 3 will read frames 1,
    /// 4, 7, and 10.
    ///
    #[clap(value_name = "STEPS", short, long, default_value = "1")]
    #[serde(default = "default_step")]
    pub step: usize,

    /// A dictionary of color attributes and their types.
    ///
    /// The dictionary string should have the following pattern:
    ///
    /// '{"color0":type0(component_type0), "color1":type1(component_type1), ..}'
    ///
    /// The color attribute names should appear exactly how they are named in
    /// the input mesh files.  On the output, these names will be converted to
    /// COLOR_# where # corresponds to the index (starting from
    /// 0) in the order they are provided on the command line.
    ///
    /// The associated types must have the format 'type(component)' where 'type'
    /// is one of [Vec3, Vec4].
    ///
    /// The component type can be one of [U8, U16, F32].
    ///
    /// which correspond to 'GL_UNSIGNED_BYTE', 'GL_UNSIGNED_SHORT', and
    /// 'GL_FLOAT' respectively.
    ///
    /// Note that component type names may be specified in lower case as well.
    ///
    /// LIMITATIONS:
    ///
    /// See LIMITATIONS section for the '--attributes' flag.
    ///
    /// EXAMPLES:
    ///
    /// The following is a valid texture coordinate attribute list:
    ///
    /// '{"diffuse": Vec3(f32), "bump": Vec3(F32)}'
    ///
    #[clap(value_name = "ATTRIBS", short, long, default_value = "{}")]
    #[serde(default)]
    pub colors: AttributeInfo,

    /// A dictionary of custom vertex attributes and their types.
    ///
    /// The dictionary string should have the following pattern:
    ///
    /// '{"attribute1":type1(component1), "attribute2":type2(component2), ..}'
    ///
    /// The attribute names should appear exactly how the attribute is named in
    /// the input mesh files.  On the output, the attribute names will be
    /// converted to SCREAMING_SNAKE case prefixed with an underscore as
    /// required by the glTF 2.0 specifications.
    ///
    /// For example an attribute named "temperatureKelvin" will be stored as
    /// "_TEMPERATURE_KELVIN" in the output. There are no guarantees for
    /// collision resolution resulting from this conversion.
    ///
    /// The associated types must have the format 'type(component)' where 'type'
    /// is one of [Scalar, Vec2, Vec3, Vec4, Mat2, Mat3, or Mat4].
    ///
    /// and 'component' is one of [I8, U8, I16, U16, U32, F32].
    ///
    /// which correspond to 'GL_BYTE', 'GL_UNSIGNED_BYTE', 'GL_SHORT',
    /// 'GL_UNSIGNED_SHORT', 'GL_UNSIGNED_INT' and 'GL_FLOAT' respectively.
    ///
    /// Scalar types may be specified without the 'Scalar(..)', but with the
    /// component type directly as 'attribute: F32' instead of 'attribute:
    /// Scalar(F32)'.
    ///
    /// Note that type and component names may be specified in all lower case as
    /// well.
    ///
    /// LIMITATIONS:
    ///
    /// Component types are not converted from the input to the output, so it's
    /// important that they are stored in the input files exactly in the types
    /// supported by glTF 2.0.  This means that double precision float attribute
    /// will not be transferred to a single precision float attribute in glTF,
    /// but will simply be ignored.
    ///
    /// EXAMPLES:
    ///
    /// The following is a valid attribute list demonstrating different ways to
    /// specify types and component types:
    ///
    /// '{"temperature":F32, "force":Vec3(F32), "material":Scalar(u32)}'
    ///
    #[clap(value_name = "ATTRIBS", short, long, default_value = "{}")]
    #[serde(default)]
    pub attributes: AttributeInfo,

    /// A dictionary of texture coordinate attributes and their types.
    ///
    /// The dictionary string should have the following pattern:
    ///
    /// '{"texcoord0":component_type1, "texcoord1":component_type2, ..}'
    ///
    /// The texture coordinate attribute names should appear exactly how they
    /// are named in the input mesh files.  On the output, these names will be
    /// converted to TEXCOORD_# where # corresponds to the index (starting from
    /// 0) in the order they are provided on the command line.
    ///
    /// The component type can be one of [U8, U16, F32].
    ///
    /// which correspond to 'GL_UNSIGNED_BYTE', 'GL_UNSIGNED_SHORT', and
    /// 'GL_FLOAT' respectively.
    ///
    /// Note that component type names may be specified in lower case as well.
    ///
    /// LIMITATIONS:
    ///
    /// See LIMITATIONS section for the '--attributes' flag.
    ///
    /// EXAMPLES:
    ///
    /// The following is a valid texture coordinate attribute list:
    ///
    /// '{"uv": f32, "bump": F32}'
    ///
    #[clap(
        value_name = "TEXCOORDS",
        short = 'u',
        long,
        default_value = "{\"uv\":f32}"
    )]
    #[serde(default)]
    pub texcoords: TextureAttributeInfo,

    /// A tuple of texture parameters.
    ///
    /// Each struct should have the following pattern:
    ///
    /// "(
    ///     image: Image,
    ///     [wrap_s: WrappingMode,]
    ///     [wrap_t: WrappingMode,]
    ///     [mag_filter: MagFilter,]
    ///     [min_filter: MinFilter,]
    /// ) .."
    ///
    /// where the fields in brackets '[]' are optional.  'Image',
    /// 'WrappingMode', 'MagFilter' and 'MinFilter' are enums (variants) that
    /// take on the following values:
    ///
    /// 'Image' is one of{n}
    ///     * Auto{n}
    ///     * Uri(path_to_image){n}
    ///     * Embed(path_to_image){n}
    ///
    /// where 'path_to_image' is the path to a 'png' or a 'jpeg' image which
    /// will be either referenced ('Uri') or embedded ('Embed') into the gltf
    /// file itself. If there is at least one image specified as 'Auto', then gltfgen
    /// will try to automatically find as many referenced images as possible in
    /// from the input. If the output is a .gltf file, then Auto images will be
    /// referenced (like 'Uri'), and if the output is .glb, then they will be embedded
    /// (like 'Embed'). If 'Auto' is used, it is recommended to keep it last in the list.
    ///
    /// The remaining optional fields describe the sampler and can take on the
    /// following values:
    ///
    /// 'WrappingMode' is one of [ClampedToEdge, MirroredRepeat, Repeat
    /// (default)].
    ///
    /// 'MagFilter' is one of [Nearest, Linear].
    ///
    /// 'MinFilter' is one of [Nearest, Linear, NearestMipmapNearest,
    /// LinearMipmapNearest, NearestMipmapLinear, or LinearMipmapLinear].
    ///
    /// See the glTF 2.0 specifications for more details
    /// https://github.com/KhronosGroup/glTF/tree/master/specification/2.0#texture-data
    ///
    /// Note that all options may be specified in snake_case as well.
    ///
    /// EXAMPLES:
    ///
    /// The following is a valid texture list:
    ///
    /// '(image: Uri("./texture.png")) (image: Embed("./texture2.png"), wrap_s:
    /// Repeat wrap_t: mirrored_repeat)'
    ///
    #[clap(value_name = "TEXTURES", short = 'x', long)]
    #[serde(default = "default_textures")]
    pub textures: Vec<TextureInfo>,

    /// A tuple of material properties.
    ///
    /// Each struct should have the following pattern:
    ///
    /// "(name:String, base_color:[f32; 4], base_texture:(index:u32,texcoord:u32),
    ///   metallic:f32, roughness:f32) .."
    ///
    /// where 'f32' indicates a single precision floating point value, and 'u32'
    /// a 32 bit unsigned integer. All fields are optional. The type '[f32; 4]'
    /// is an array of 4 floats corresponding to red, green, blue and alpha
    /// values between 0.0 and 1.0. 'metallic' and 'roughness' factors are
    /// expected to be between 0.0 and 1.0.
    ///
    /// 'base_texture' specifies the texture to be used by the material. 'index' specifies the
    /// 0-based index of the texture provided by the '--textures' (or '-x') flag. 'texcoord'
    /// specifies the index of the texture attribute specified by the '--texcoords' (or '-u') flag.
    /// 'base_texture' is not set by default.
    ///
    /// Default values are 0.0 for 'metallic', 0.5 for 'roughness', and [0.5, 0.5,
    /// 0.5, 1.0] for 'base_color'.
    ///
    /// If a texture is specified with the -x or --textures flag in 'Auto' mode
    /// (default), then gltfgen will create a default binding to each 'Auto'
    /// image found. Note that the 'index' specified in 'base_texture' will be
    /// in the order the 'Auto' images are found, which is unspecified.
    /// This means that it is best to place the 'Auto' image reference at the
    /// end of the list if used.
    ///
    /// EXAMPLES:
    ///
    /// The following are examples of valid material specifications:
    ///
    /// "()"
    ///
    /// produces a default material.
    ///
    /// '(name:"material0", base_color:[0.1, 0.2, 0.3, 1.0], metallic:0.0)'
    ///
    /// produces a material named "material0" with the specified base_color and
    /// metallic factor.
    ///
    #[clap(value_name = "MATERIALS", short, long)]
    #[serde(default)]
    pub materials: Vec<MaterialInfo>,

    /// Name of the material attribute on mesh faces or cells.
    ///
    /// This is used for determining which materials should be assigned to which meshes.
    ///
    /// This attribute must be an integer (at most 64 bit) and must index materials specified by
    /// the '-m' or '--materials' flag.
    ///
    #[clap(value_name = "MTL-ATTRIB", short = 'e', long, default_value = "mtl_id")]
    #[serde(default = "default_mtl_id")]
    pub material_attribute: String,
}
