use std::io::BufReader;
use std::path::PathBuf;

use clap::Parser;
use console::style;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use thiserror::Error; // For colouring log messages.

use gltfgen::log;
use gltfgen::*;

const ABOUT: &str = "
gltfgen generates gltf files in standard and binary formats from a given sequence of mesh files.";

const EXAMPLES: &str = "
EXAMPLES:

The following examples assume that there is a sequence of meshes located at \
`./meshes/animation_#.vtk` where `#` represents the frame number and an image \
located in `./texture.png` to be used as a texture.

To generate an animated binary glTF file named `output.glb` in the current directory:

$ gltfgen output.glb \"./meshes/animation_#.vtk\"


This will assume 24 frames per second (FPS). You can specify FPS manually with the `-f` option as \
follows:

$ gltfgen -f 100 output.glb \"./meshes/animation_#.vtk\"


Alternatively, to specify a time step like 0.01 seconds between frames, use the `-t` option:

$ gltfgen -t 0.01 output.glb \"./meshes/animation_#.vtk\"


To add color to the glTF from an attribute in the vtk file use the `-c` option:

$ gltfgen -c '{\"Cd\":vec3(f32)}' output.glb \"./meshes/animation_#.vtk\"


To add texture to the output glTF, use the `-u` option to specify texture coordinates, \
use the `-x` option to specify the image to be used, and the `-m` option to create a \
material that binds the image to the texture coordinates:

$ gltfgen -u '{\"uv\":f32}' \\
          -x '(image: Embed(\"./texture.png\")` \\
          -m '(name:\"texture\", base_texture:{index:0,texcoord:0)) \\
          output.glb \"./meshes/animation_#.vtk\"
";

#[derive(Parser, Debug)]
#[clap(author, version, about = ABOUT, name = "gltfgen")]
#[clap(after_long_help(EXAMPLES))]
struct Opt {
    /// Output glTF file.
    #[clap(name = "OUTPUT", parse(from_os_str))]
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
    #[clap(name = "PATTERN", parse(from_str))]
    pattern: String,

    /// Silence all output.
    #[clap(short, long)]
    quiet: bool,

    #[clap(flatten)]
    config: config::Config,

    /// A path to the configuration file specifying how glTF files should be built.
    ///
    /// If specified, all command line configuration is ignored.
    #[clap(name = "CONFIG", long = "config", parse(from_os_str))]
    config_path: Option<PathBuf>,
}

#[derive(Debug, Error)]
enum Error {
    #[error("{}", .0)]
    Glob(#[from] glob::GlobError),
    #[error("{}", .0)]
    GlobPattern(#[from] glob::PatternError),
    #[error("No valid meshes were found")]
    NoMeshesFound,
    #[error("Configuration load error: {}", .0)]
    ConfigLoad(#[from] std::io::Error),
    #[error("Configuration deserialization error: {}", .0)]
    ConfigDeserialize(#[from] ron::Error),
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        std::process::exit(1); // Non-zero value indicating that an error occurred.
    }
}
fn try_main() -> Result<(), Error> {
    let opt = Opt::parse();

    // Try to load the config file if specified.
    let config = if let Some(path) = opt.config_path {
        use std::fs::File;
        File::open(path).map_err(Error::from).and_then(|f| {
            let reader = BufReader::new(f);
            Ok(ron::de::from_reader(reader)?)
        })?
    } else {
        opt.config
    };

    let pattern = if opt.pattern.starts_with("./") {
        &opt.pattern[2..]
    } else {
        &opt.pattern[..]
    };

    let regex = glob_to_regex(pattern);
    let pattern = remove_braces(
        &pattern
            .replace("*#*", "*")
            .replace("*#", "*")
            .replace("#*", "*")
            .replace('#', "*"),
    );
    let glob_options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let pb = utils::new_spinner(opt.quiet);

    pb.set_prefix("Looking for files");

    let entries = glob::glob_with(&pattern, glob_options)?;

    // First parse entries and retrieve the necessary data before building the meshes.
    // This will allow us to prune skipped frames before actually building meshes.

    let mut lowest_frame_num = None;

    let mut warnings = Vec::new();

    let mut mesh_meta: Vec<_> = entries
        .filter_map(|entry| {
            entry.ok().and_then(|path| {
                pb.tick();
                if let Some(f) = path.file_name() {
                    pb.set_message(f.to_string_lossy().into_owned());
                }
                let path_str = path.to_string_lossy();
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
                for cap in caps
                    .iter()
                    .skip(1)
                    .filter(|&cap| cap != frame_cap)
                    .flatten()
                {
                    name.push_str(cap.as_str());
                }
                Some((name, frame, path))
            })
        })
        .collect();

    pb.finish_with_message(format!("Found {} files", mesh_meta.len()));

    print_warnings(warnings);

    // Prune mesh meta before building meshes
    if config.step > 1 {
        if let Some(lowest_frame_num) = lowest_frame_num {
            let pb = utils::new_progress_bar(opt.quiet, mesh_meta.len());
            pb.set_message("Pruning frames");

            mesh_meta = mesh_meta
                .into_par_iter()
                .progress_with(pb.clone())
                .filter_map(|(name, frame, path)| {
                    // Note frameless meshes are placed at frame zero, and they won't be skipped
                    // here.
                    if (frame - lowest_frame_num) % config.step == 0 {
                        Some((name, frame, path))
                    } else {
                        None
                    }
                })
                .collect();

            pb.finish_with_message(format!("{} frames remain after pruning", mesh_meta.len()));
        }
    }

    let pb = utils::new_progress_bar(opt.quiet, mesh_meta.len());
    pb.set_message("Building Meshes");

    let load_config = LoadConfig {
        attributes: &config.attributes,
        colors: &config.colors,
        texcoords: &config.texcoords,
        material_attribute: &config.material_attribute,
        reverse: config.reverse,
        invert_tets: config.invert_tets,
    };

    let process_attrib_error = |e| {
        pb.println(format!("{}: {}, Skipping...", style("WARNING").yellow(), e));
    };

    // Load all meshes with the appropriate conversions and attribute transfers.
    let meshes: Vec<_> = mesh_meta
        .into_par_iter()
        .progress_with(pb.clone())
        .filter_map(|(name, frame, path)| {
            load_mesh(&path, load_config, process_attrib_error)
                .map(|(mesh, attrib_transfer)| (name, frame, mesh, attrib_transfer))
        })
        .collect();

    pb.finish_with_message("Done building meshes");

    let dt = if let Some(dt) = config.time_step {
        dt
    } else {
        1.0 / config.fps as f32
    };

    if meshes.is_empty() {
        return Err(Error::NoMeshesFound);
    }

    export::export(
        meshes,
        opt.output,
        dt,
        opt.quiet,
        config.textures,
        config.materials,
    );

    Ok(())
}
