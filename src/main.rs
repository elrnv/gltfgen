mod export;
mod utils;

use utils::*;

use std::fmt;
use std::path::PathBuf;
use structopt::StructOpt;

use geo::mesh::TriMesh;

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
    reverse: bool
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
    let opt = Opt::from_args();
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
    let mut meshes = Vec::new();
    let glob_options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    for entry in glob::glob_with(&pattern, glob_options)?
    {
        match entry {
            Ok(path) => {
                let path_str = path.to_string_lossy();
                let caps = regex.captures(&path_str).expect(&format!(
                    "ERROR: Regex '{}' did not match path '{}'",
                    regex.as_str(),
                    &path_str
                ));
                let frame_cap = caps.name("frame").unwrap();
                let frame = frame_cap
                    .as_str()
                    .parse::<usize>()
                    .expect("ERROR: Failed to parse frame number");

                // Find a unique name for this mesh in the filename.
                let mut name = String::new();
                for cap in caps.iter().skip(1).filter(|&cap| cap != Some(frame_cap)) {
                    if let Some(cap) = cap {
                        name.push_str(cap.as_str());
                    }
                }

                let mut mesh = if let Ok(polymesh) = geo::io::load_polymesh::<f64, _>(&path) {
                    trimesh_f64_to_f32(TriMesh::from(polymesh))
                } else if let Ok(polymesh) = geo::io::load_polymesh::<f32, _>(&path) {
                    TriMesh::<f32>::from(polymesh)
                } else if let Ok(tetmesh) = geo::io::load_tetmesh::<f64, _>(&path) {
                    trimesh_f64_to_f32(tetmesh.surface_trimesh())
                } else if let Ok(tetmesh) = geo::io::load_tetmesh::<f32, _>(path) {
                    tetmesh.surface_trimesh()
                } else {
                    continue;
                };

                if opt.reverse {
                    mesh.reverse();
                }

                meshes.push((name, frame, mesh));
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
    let dt = if let Some(fps) = opt.fps {
        1.0 / fps as f32
    } else {
        opt.time_step
    };
    export::export(meshes, opt.output, dt);
    Ok(())
}
