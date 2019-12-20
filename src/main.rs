#[macro_use]
mod attrib;
mod export;
mod material;
mod texture;
mod utils;

use attrib::*;
use material::*;
use texture::*;
use utils::*;

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use structopt::StructOpt;

use gut::mesh::TriMesh;

use rayon::prelude::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "gltfgen")]
struct Opt {
    /// Output glTF file
    #[structopt(parse(from_os_str))]
    output: PathBuf,

    /// A glob pattern matching files to be included in the generated glTF document.
    ///
    /// Use # to match a frame number. If more than one '#' is used, the first match will
    /// correspond to the frame number. Note that the glob pattern should generally by provided
    /// as a quoted string to prevent the terminal from evaluating it.
    ///
    /// Strings within between braces (i.e. '{' and '}') will be used as names for unique
    /// animations.
    /// This means that a single output can contain multiple animations. If more than one group is
    /// specified, the matched strings within will be concatenated to produce a unique name.
    /// Note that for the time being, '{' '}' are ignored when the glob pattern is matched.
    #[structopt(parse(from_str))]
    pattern: String,

    /// Frames per second. 1.0/fps gives the time step between discrete frames.
    #[structopt(short, long)]
    fps: Option<usize>,

    /// Time step in seconds between discrete frames. If 'fps' is also provided, this parameter is
    /// ignored.
    #[structopt(short, long, default_value = "1.0")]
    time_step: f32,

    /// Reverse polygon orientations in the output glTF meshes.
    #[structopt(short, long)]
    reverse: bool,

    /// Invert tetrahedra orientations on the input meshes.
    #[structopt(short, long)]
    invert_tets: bool,

    /// Silence all output.
    #[structopt(short, long)]
    quiet: bool,

    /// Step by this number of frames.
    ///
    /// In other words, read frames in increments of 'step'.
    /// Note that this does not affect the value for 'fps' or 'time_step' options.
    /// This number must be at least 1.
    ///
    /// For example for frames 1 to 10, a 'step' value of 3 will read frames 1, 4, 7, and 10.
    ///
    #[structopt(short, long, default_value = "1")]
    step: usize,

    /// A list of custom vertex attributes and their types to transfer provided in a common type
    /// ascription syntax:
    ///
    /// "attribute1: type1<component_type1>" "attribute2: type2<component_type2>"
    ///
    /// Note: Quotation marks are necessary to escale the < and > symbols. It may also be
    ///       necessary to specify this option last.
    ///
    /// The attribute names should appear exactly how the attribute is named in the input mesh
    /// files.  On the output, the attribute names will be converted to SCREAMING_SNAKE case
    /// prefixed with an underscore as required by the glTF 2.0 specifications.
    ///
    /// For example an attribute named "temperatureKelvin" will be stored as "_TEMPERATURE_KELVIN"
    /// in the output. There are no guarantees for collision resolution resulting from this
    /// conversion.
    ///
    /// The associated types must have the format type<component> where 'type' is one of
    ///
    ///     Scalar, Vec2, Vec3, Vec4, Mat2, Mat3, or Mat4
    ///
    /// and 'component' is one of
    ///
    ///     i8, u8, i16, u16, u32, or f32
    ///
    /// which correspond to 'GL_BYTE', 'GL_UNSIGNED_BYTE', 'GL_SHORT', 'GL_UNSIGNED_SHORT',
    /// 'GL_UNSIGNED_INT' and 'GL_FLOAT' respectively.
    ///
    /// Note that type and component names are not case sensitive.
    ///
    /// EXAMPLE
    ///
    /// The following is a valid attribute list:
    ///
    /// "temperature: Scalar<f32>" "force: Vec3<f32>" "material: Scalar<u32>"
    ///
    #[structopt(short, long)]
    attributes: Vec<AttributeInfo>,

    /// A list of texture coordinate attributes and their types to transfer provided in a common
    /// type ascription syntax:
    ///
    /// "texcoord0: component_type1" "texcoord1: component_type2"
    ///
    /// The texture coordinate attribute names should appear exactly how they are named in the
    /// input mesh files.  On the output, these names will be converted to TEXCOORD_# where #
    /// corresponds to the index (starting from 0) in the order they are provided on the command
    /// line.
    ///
    /// The component type can be one of
    ///
    ///     u8, u16, or f32
    ///
    /// which correspond to 'GL_UNSIGNED_BYTE', 'GL_UNSIGNED_SHORT', and 'GL_FLOAT' respectively.
    ///
    /// Note that component type names are not case sensitive.
    ///
    /// EXAMPLE
    ///
    /// The following is a valid texture coordinate attribute list:
    ///
    /// "uv: f32" "bump: f32"
    ///
    #[structopt(short = "uv", long)]
    texcoords: Vec<TextureAttributeInfo>,

    /// Specify textures in Rusty Object Notation (RON) (https://github.com/ron-rs/ron) as a list
    /// of structs:
    ///
    /// "(
    ///     image: Image,
    ///     [wrap_s: WrappingMode,]
    ///     [wrap_t: WrappingMode,]
    ///     [mag_filter: MagFilter,]
    ///     [min_filter: MinFilter,]
    /// ) .."
    ///
    /// where the fields in brackets '[]' represent optional fields.
    /// 'Image', 'WrappingMode', 'MagFilter' and 'MinFilter' are enums (variants) that take on the
    /// following values:
    ///
    /// 'Image' is one of
    ///     * Uri(path_to_image)
    ///     * Embed(path_to_image)
    ///
    /// where 'path_to_image' is the path to a 'png' or a 'jpeg' image which will be either
    /// referenced ('Uri') or embedded ('Embed') into the gltf file itself.
    ///
    /// The remaining optional fields describe the sampler and can take on the following values:
    ///
    /// 'WrappingMode' is one of
    ///     * ClampedToEdge
    ///     * MirroredRepeat
    ///     * Repeat (default)
    ///
    /// 'MagFilter' is one of
    ///     * Nearest
    ///     * Linear
    ///
    /// 'MinFilter' is one of
    ///     * Nearest
    ///     * Linear
    ///     * NearestMipmapNearest
    ///     * LinearMipmapNearest
    ///     * NearestMipmapLinear
    ///     * LinearMipmapLinear
    ///
    /// See the glTF 2.0 specifications for more details
    /// https://github.com/KhronosGroup/glTF/tree/master/specification/2.0#texture-data
    ///
    /// EXAMPLE
    ///
    /// The following is a valid texture list:
    ///
    /// "(image: Uri(\"./texture.png\")) (image: Embed(\"./texture2.png\"), wrap_s: Repeat)"
    ///
    #[structopt(short = "x", long)]
    textures: Vec<TextureInfo>,

    /// Specify material properties in Rusty Object Notation (RON) (https://github.com/ron-rs/ron)
    /// as a list of structs:
    ///
    /// "(
    ///     [name: String,]
    ///     [base_color: [f32; 4],]
    ///     [base_texture: (index: u32, texcoord: u32),]
    ///     [metallic: f32,]
    ///     [roughness: f32,]
    /// ) .."
    ///
    #[structopt(short, long)]
    materials: Vec<MaterialInfo>,

    /// Specify the name of the material attribute to use for determining which materials should be
    /// assigned to which meshes.
    ///
    /// The material attribute must reside on primitives (i.e. polygons on polygon meshes and
    /// tetrahedra on tetrahedron meshes).
    ///
    /// This attribute must be an unsigned 32 bit integer and must index materials specified by the
    /// '-m' or '--materials' flag.
    ///
    #[structopt(short = "ma", long, default_value = "mtl_id")]
    material_attribute: String,
}

#[derive(Debug)]
enum Error {
    GlobError(glob::GlobError),
    GlobPatternError(glob::PatternError),
}

impl From<glob::GlobError> for Error {
    fn from(glob_err: glob::GlobError) -> Error {
        Error::GlobError(glob_err)
    }
}

impl From<glob::PatternError> for Error {
    fn from(src: glob::PatternError) -> Error {
        Error::GlobPatternError(src)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::GlobError(e) => e.fmt(f),
            Error::GlobPatternError(e) => e.fmt(f),
        }
    }
}

fn main() -> Result<(), Error> {
    let mut opt = Opt::from_args();

    // Correct texture coordinate attribute ids.
    opt.texcoords
        .iter_mut()
        .enumerate()
        .for_each(|(i, tex_attrib_info)| tex_attrib_info.id = i as u32);

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

    if !opt.quiet {
        println!("Preprocessing Files...");
    }
    let entries: Vec<_> = glob::glob_with(&pattern, glob_options)?.collect();

    // First parse entries and retrieve the necessary data before building the meshes.
    // This will allow us to prune skipped frames before actually building meshes.

    let mut lowest_frame_num = None;

    let mut mesh_meta: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| {
            entry.ok().map(|path| {
                let path_str = path.to_string_lossy();
                let caps = match regex.captures(&path_str) {
                    Some(caps) => caps,
                    None => {
                        eprintln!(
                            "WARNING: Path '{}' skipped since regex '{}' did not match.",
                            &path_str,
                            regex.as_str(),
                        );
                        return None;
                    },
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
                (name, frame, path)
            })
        })
        .collect();

    // Prune mesh meta before building meshes
    if opt.step > 1 {
        if let Some(lowest_frame_num) = lowest_frame_num {
            mesh_meta = mesh_meta
                .into_par_iter()
                .filter_map(|(name, frame, path)| {
                    // Note frameless meshes are placed at frame zero, and they won't be skipped
                    // here.
                    if (frame - lowest_frame_num) % opt.step == 0 {
                        Some((name, frame, path))
                    } else {
                        None
                    }
                })
                .collect()
        }
    }

    let pb = Arc::new(RwLock::new(pbr::ProgressBar::new(mesh_meta.len() as u64)));
    if !opt.quiet {
        pb.write().unwrap().message("Building Meshes ")
    }

    let meshes: Vec<_> = mesh_meta
        .into_par_iter()
        .filter_map(|(name, frame, path)| {
            if !opt.quiet {
                pb.write().unwrap().inc();
            }
            let mut mesh = if let Ok(polymesh) = gut::io::load_polymesh::<f64, _>(&path) {
                trimesh_f64_to_f32(TriMesh::from(polymesh))
            } else if let Ok(polymesh) = gut::io::load_polymesh::<f32, _>(&path) {
                TriMesh::<f32>::from(polymesh)
            } else if let Ok(tetmesh) = gut::io::load_tetmesh::<f64, _>(&path) {
                let mut trimesh = tetmesh.surface_trimesh();
                if opt.invert_tets {
                    trimesh.reverse();
                }
                trimesh_f64_to_f32(trimesh)
            } else if let Ok(tetmesh) = gut::io::load_tetmesh::<f32, _>(path) {
                let mut trimesh = tetmesh.surface_trimesh();
                if opt.invert_tets {
                    trimesh.reverse();
                }
                trimesh
            } else {
                return None;
            };

            if opt.reverse {
                mesh.reverse();
            }

            let attrib_transfer = clean_attributes(
                &mut mesh,
                &opt.attributes,
                &opt.texcoords,
                &opt.material_attribute,
            );

            Some((name, frame, mesh, attrib_transfer))
        })
        .collect();
    if !opt.quiet {
        pb.write().unwrap().finish();
    }
    let dt = if let Some(fps) = opt.fps {
        1.0 / fps as f32
    } else {
        opt.time_step
    };

    if !opt.quiet {
        println!("Exporting glTF...");
    }
    export::export(
        meshes,
        opt.output,
        dt,
        opt.quiet,
        opt.textures,
        opt.materials,
    );
    if !opt.quiet {
        println!("Success!");
    }
    Ok(())
}
