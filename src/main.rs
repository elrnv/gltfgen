use std::path::PathBuf;

use console::style;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use structopt::StructOpt;
use thiserror::Error; // For colouring log messages.

use gltfgen::log;
use gltfgen::*;

const ABOUT: &str = "
gltfgen generates gltf files in standard and binary formats from a given sequence of mesh files.";

#[derive(StructOpt, Debug)]
#[structopt(author, about = ABOUT, name = "gltfgen")]
struct Opt {
    /// Output glTF file
    #[structopt(name = "OUTPUT", parse(from_os_str))]
    output: PathBuf,
    /// A glob pattern matching input mesh files.
    ///
    /// Use # to match a frame number. If more than one '#' is used, the first
    /// match will correspond to the frame number. Note that the glob pattern
    /// should generally by provided as a quoted string to prevent the terminal
    /// from evaluating it.
    ///
    /// Strings within between braces (i.e. '{' and '}') will be used as names
    /// for unique animations.  This means that a single output can contain
    /// multiple animations. If more than one group is specified, the matched
    /// strings within will be concatenated to produce a unique name.  Note that
    /// for the time being, '{' '}' are ignored when the glob pattern is
    /// matched.
    #[structopt(name = "PATTERN", parse(from_str))]
    pattern: String,

    /// Frames per second.
    ///
    /// 1/fps gives the time step between discrete frames. If 'time_step' is also provided, this
    /// parameter is ignored.
    #[structopt(value_name = "FPS", short, long, default_value = "24")]
    fps: usize,

    /// Time step in seconds between discrete frames.
    ///
    /// Specifying this option overrides the time step that would be computed from 'fps', which is
    /// set to 24 by default.  This means that the default 'time_step' is equivalently 1/24.
    #[structopt(value_name = "TIMESTEP", short, long)]
    time_step: Option<f32>,

    /// Reverse polygon orientations on output meshes.
    #[structopt(short, long)]
    reverse: bool,

    /// Invert tetrahedra orientations on input meshes.
    #[structopt(short, long)]
    invert_tets: bool,

    /// Silence all output.
    #[structopt(short, long)]
    quiet: bool,

    /// Step by the given number of frames.
    ///
    /// In other words, read frames in increments of 'step'.  Note that this
    /// does not affect the value for 'fps' or 'time_step' options.  This number
    /// must be at least 1.
    ///
    /// For example for frames 1 to 10, a 'step' value of 3 will read frames 1,
    /// 4, 7, and 10.
    ///
    #[structopt(value_name = "STEPS", short, long, default_value = "1")]
    step: usize,

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
    /// LIMITATIONS
    ///
    /// See LIMITATIONS section for the '--attributes' flag.
    ///
    /// EXAMPLES
    ///
    /// The following is a valid texture coordinate attribute list:
    ///
    /// '{"diffuse": Vec3(f32), "bump": Vec3(F32)}'
    ///
    #[structopt(value_name = "ATTRIBS", short, long, default_value = "{}")]
    colors: AttributeInfo,

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
    /// LIMITATIONS
    ///
    /// Component types are not converted from the input to the output, so it's
    /// important that they are stored in the input files exactly in the types
    /// supported by glTF 2.0.  This means that double precision float attribute
    /// will not be transferred to a single precision float attribute in glTF,
    /// but will simply be ignored.
    ///
    /// EXAMPLES
    ///
    /// The following is a valid attribute list demonstrating different ways to
    /// specify types and component types:
    ///
    /// '{"temperature":F32, "force":Vec3(F32), "material":Scalar(u32)}'
    ///
    #[structopt(value_name = "ATTRIBS", short, long, default_value = "{}")]
    attributes: AttributeInfo,

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
    /// LIMITATIONS
    ///
    /// See LIMITATIONS section for the '--attributes' flag.
    ///
    /// EXAMPLES
    ///
    /// The following is a valid texture coordinate attribute list:
    ///
    /// '{"uv": f32, "bump": F32}'
    ///
    #[structopt(value_name = "TEXCOORDS", short = "u", long, default_value = "{}")]
    texcoords: TextureAttributeInfo,

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
    /// where the fields in brackets '[]' represent optional fields.  'Image',
    /// 'WrappingMode', 'MagFilter' and 'MinFilter' are enums (variants) that
    /// take on the following values:
    ///
    /// 'Image' is one of * Uri(path_to_image) * Embed(path_to_image)
    ///
    /// where 'path_to_image' is the path to a 'png' or a 'jpeg' image which
    /// will be either referenced ('Uri') or embedded ('Embed') into the gltf
    /// file itself.
    ///
    /// The remaining optional fields describe the sampler and can take on the
    /// following values:
    ///
    /// 'WrappingMode' is one of [ClampedToEdge, MirroredRepeat, Repeat (default)].
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
    /// EXAMPLES
    ///
    /// The following is a valid texture list:
    ///
    /// '(image: Uri("./texture.png")) (image: Embed("./texture2.png"), wrap_s:
    /// Repeat wrap_t: mirrored_repeat)'
    ///
    #[structopt(value_name = "TEXTURES", short = "x", long)]
    textures: Vec<TextureInfo>,

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
    /// Default values are 0.0 for 'metallic', 0.5 for 'roughness', and [0.5, 0.5,
    /// 0.5, 1.0] for 'base_color.
    ///
    /// EXAMPLES
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
    #[structopt(value_name = "MATERIALS", short, long)]
    materials: Vec<MaterialInfo>,

    /// Name of the material attribute on mesh faces or cells.
    ///
    /// This is used for determining which materials should be assigned to which meshes.
    ///
    /// This attribute must be an integer (at most 64 bit) and must index materials specified by
    /// the '-m' or '--materials' flag.
    ///
    #[structopt(value_name = "MTL-ATTRIB", short = "e", long, default_value = "mtl_id")]
    material_attribute: String,
}

#[derive(Debug, Error)]
enum Error {
    #[error("{}", .0)]
    Glob(#[from] glob::GlobError),
    #[error("{}", .0)]
    GlobPattern(#[from] glob::PatternError),
    #[error("No valid meshes were found")]
    NoMeshesFound,
}

fn main() -> Result<(), Error> {
    use terminal_size::{terminal_size, Width};
    let app = Opt::clap().set_term_width(if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    });

    let opt = Opt::from_clap(&app.get_matches());

    let pattern = if opt.pattern.starts_with("./") {
        &opt.pattern[2..]
    } else {
        &opt.pattern[..]
    };

    let regex = glob_to_regex(&pattern);
    let pattern = remove_braces(
        &pattern
            .replace("*#*", "*")
            .replace("*#", "*")
            .replace("#*", "*")
            .replace("#", "*"),
    );
    let glob_options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let pb = utils::new_spinner(opt.quiet);

    pb.set_prefix("Looking for files");

    let entries: Vec<_> = glob::glob_with(&pattern, glob_options)?.collect();

    // First parse entries and retrieve the necessary data before building the meshes.
    // This will allow us to prune skipped frames before actually building meshes.

    let mut lowest_frame_num = None;

    let mut warnings = Vec::new();

    let mut mesh_meta: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| {
            entry.ok().and_then(|path| {
                pb.tick();
                let path_str = path.to_string_lossy();
                if let Some(f) = path.file_name() {
                    pb.set_message(&f.to_string_lossy());
                }
                let caps = match regex.captures(&path_str) {
                    Some(caps) => caps,
                    None => {
                        log!(warnings;
                            "Path '{}' skipped since regex '{}' did not match.",
                            &path_str,
                            regex.as_str(),
                        );
                        return None;
                    }
                };
                let frame_cap = caps.name("frame");
                let frame = frame_cap
                    .map(|frame_match| {
                        let frame = frame_match
                            .as_str()
                            .parse::<usize>()
                            .expect("ERROR: Failed to parse frame number");
                        lowest_frame_num =
                            Some(lowest_frame_num.map_or(frame, |n: usize| n.min(frame)));
                        frame
                    })
                    .unwrap_or(0);

                // Find a unique name for this mesh in the filename.
                let mut name = String::new();
                for cap in caps.iter().skip(1).filter(|&cap| cap != frame_cap) {
                    if let Some(cap) = cap {
                        name.push_str(cap.as_str());
                    }
                }
                Some((name, frame, path))
            })
        })
        .collect();

    pb.finish_with_message(&format!("Found {} files", mesh_meta.len()));

    print_warnings(warnings);

    // Prune mesh meta before building meshes
    if opt.step > 1 {
        if let Some(lowest_frame_num) = lowest_frame_num {
            let pb = utils::new_progress_bar(opt.quiet, mesh_meta.len());
            pb.set_message("Pruning frames");

            mesh_meta = mesh_meta
                .into_par_iter()
                .progress_with(pb.clone())
                .filter_map(|(name, frame, path)| {
                    // Note frameless meshes are placed at frame zero, and they won't be skipped
                    // here.
                    if (frame - lowest_frame_num) % opt.step == 0 {
                        Some((name, frame, path))
                    } else {
                        None
                    }
                })
                .collect();

            pb.finish_with_message(&format!("{} frames remain after pruning", mesh_meta.len()));
        }
    }

    let pb = utils::new_progress_bar(opt.quiet, mesh_meta.len());
    pb.set_message("Building Meshes");

    let config = LoadConfig {
        attributes: &opt.attributes,
        colors: &opt.colors,
        texcoords: &opt.texcoords,
        material_attribute: &opt.material_attribute,
        reverse: opt.reverse,
        invert_tets: opt.invert_tets,
    };

    let process_attrib_error = |e| {
        pb.println(format!("{}: {}, Skipping...", style("WARNING").yellow(), e));
    };

    // Load all meshes with the appropriate conversions and attribute transfers.
    let meshes: Vec<_> = mesh_meta
        .into_par_iter()
        .progress_with(pb.clone())
        .filter_map(|(name, frame, path)| {
            load_mesh(&path, config, process_attrib_error)
                .map(|(mesh, attrib_transfer)| (name, frame, mesh, attrib_transfer))
        })
        .collect();

    pb.finish_with_message("Done building meshes");

    let dt = if let Some(dt) = opt.time_step {
        dt
    } else {
        1.0 / opt.fps as f32
    };

    if meshes.is_empty() {
        return Err(Error::NoMeshesFound);
    }

    export::export(
        meshes,
        opt.output,
        dt,
        opt.quiet,
        opt.textures,
        opt.materials,
    );

    Ok(())
}
