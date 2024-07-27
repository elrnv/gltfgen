use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use console::style;
use env_logger;
use gltfgen::config::Config;
use indicatif::ParallelProgressIterator;
use log;
use rayon::prelude::*;

use gltfgen::*;

const ABOUT: &str = "
gltfgen generates gltf files in standard and binary formats from a given sequence of mesh files.";

const EXAMPLES: &str = "
EXAMPLES:

The following examples assume that there is a sequence of meshes located at \
`./meshes/animation_#.vtk` where `#` represents the frame number and an image \
located in `./texture.png` to be used as a texture.

To generate an animated binary glTF file named `output.glb` in the current directory:

$ gltfgen -o output.glb \"./meshes/animation_#.vtk\"


This will assume 24 frames per second (FPS). You can specify FPS manually with the `-f` option as \
follows:

$ gltfgen -f 100 -o output.glb \"./meshes/animation_#.vtk\"


Alternatively, to specify a time step like 0.01 seconds between frames, use the `-t` option:

$ gltfgen -t 0.01 -o output.glb \"./meshes/animation_#.vtk\"


To add color to the glTF from an attribute in the vtk file use the `-c` option:

$ gltfgen -c '{\"Cd\":vec3(f32)}' -o output.glb \"./meshes/animation_#.vtk\"


To add texture to the output glTF, use the `-u` option to specify texture coordinates, \
use the `-x` option to specify the image to be used, and the `-m` option to create a \
material that binds the image to the texture coordinates:

$ gltfgen -u '{\"uv\":f32}' \\
          -x '(image: Embed(\"./texture.png\")' \\
          -m '(name:\"texture\", base_texture:(index:0,texcoord:0))' \\
          -o output.glb \"./meshes/animation_#.vtk\"

NOTE: In PowerShell, double quotes should be doubled.
";

#[derive(Parser, Debug)]
#[clap(author, version, about = ABOUT, name = "gltfgen")]
#[clap(after_long_help(EXAMPLES))]
struct Opt {
    #[clap(flatten)]
    config: config::Config,

    /// A path to the configuration file specifying how glTF files should be built.
    ///
    /// If unspecified, gltfgen will look for a 'gltfgen.ron' or a
    /// 'gltfgen.json' configuration file in the current working directory, and
    /// if none found, it will use default arguments or arguments specified
    /// on the command line.
    ///
    /// If specified, any explicit command line configuration will override the config loaded.
    #[clap(name = "CONFIG", long = "config")]
    config_path: Option<PathBuf>,

    /// Controls verobosity of printed output.
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,

    /// Print the configuration in JSON format, but don't run the generator.
    ///
    /// This is useful for debugging or for generating a configuration file that
    /// can later be reused. For example, run
    ///
    /// $ gltfgen -f 60 --print-json-config > gltfgen.json
    ///
    /// to create a JSON configuration file initialized with default parameters and 60 fps output.
    /// This file can then later be used as
    ///
    /// $ gltfgen --config gltfgen.json "<PATTERN>"
    ///
    /// without specifying the '-f' flag explicitly every time. (Replace
    /// <PATTERN> with an actual sequence pattern).
    ///
    /// The --verbose and --quiet flags are ignored.
    #[clap(long)]
    print_json_config: bool,

    /// Print the configuration in RON format, but don't run the generator.
    ///
    /// This is useful for debugging or for generating a configuration file that
    /// can later be reused. For example, run
    ///
    /// $ gltfgen -f 60 --print-ron-config > gltfgen.ron
    ///
    /// to create a RON configuration file initialized with default parameters and 60 fps output.
    /// This file can then later be used as
    ///
    /// $ gltfgen --config gltfgen.ron "<PATTERN>"
    ///
    /// without specifying the '-f' flag explicitly every time. (Replace
    /// <PATTERN> with an actual sequence pattern).
    ///
    /// The --verbose and --quiet flags are ignored.
    #[clap(long)]
    print_ron_config: bool,

    /// Print the configuration and required input argments, but don't run the generator.
    ///
    /// This is only useful for debugging. For printing the config using a
    /// specific format like JSON or RON, use either of --print-json-config or
    /// --print-ron-config options.
    ///
    /// The --verbose and --quiet flags are ignored.
    #[clap(long)]
    print_full_config: bool,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        std::process::exit(1); // Non-zero value indicating that an error occurred.
    }
}
fn try_main() -> Result<(), Error> {
    let cli = clap::Command::new("gltfgen");
    let cli = <Opt as clap::Args>::augment_args(cli);
    let matches = cli.get_matches();
    let opt = <Opt as clap::FromArgMatches>::from_arg_matches(&matches).unwrap();
    env_logger::Builder::new()
        .filter_level(opt.verbose.log_level_filter())
        .init();

    // Try to load the config file if specified.
    let config = if let Some(path) = opt.config_path {
        Config::load_with_override(path, &opt.config, &matches)?
    } else {
        // Check if there is a local configuration file with the name "gltfgen.ron" or "gltfgen.json" and try to load that.
        if let Ok(local_config) = Config::load_with_override("./gltfgen.ron", &opt.config, &matches)
        {
            print_info(vec![(1, "Using local ./gltfgen.ron config.".to_string())]);
            local_config
        } else if let Ok(local_config) =
            Config::load_with_override("./gltfgen.json", &opt.config, &matches)
        {
            print_info(vec![(1, "Using local ./gltfgen.json config.".to_string())]);
            local_config
        } else {
            // Otherwise just use whatever was specified on the commandline.
            opt.config
        }
    };

    if opt.print_full_config {
        println!("{:#?}", config);
        return Ok(());
    } else if opt.print_ron_config {
        println!(
            "{}",
            ron::ser::to_string_pretty(&config, ron::ser::PrettyConfig::default())?
        );
        return Ok(());
    } else if opt.print_json_config {
        println!("{}", serde_json::to_string(&config)?);
        return Ok(());
    }

    let pattern = if config.pattern.starts_with("./") {
        &config.pattern[2..]
    } else {
        &config.pattern[..]
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

    let pb = utils::new_spinner(opt.verbose.is_silent());

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
                        crate::log!(warnings;
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
                            .parse::<u32>()
                            .expect("ERROR: Failed to parse frame number");
                        lowest_frame_num =
                            Some(lowest_frame_num.map_or(frame, |n: u32| n.min(frame)));
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

    log::warn!("Glob returned {} entries", mesh_meta.len());

    pb.finish_with_message(format!("Found {} files", mesh_meta.len()));

    print_warnings(warnings);

    // Prune mesh meta before building meshes
    if config.step > 1 {
        if let Some(lowest_frame_num) = lowest_frame_num {
            let pb = utils::new_progress_bar(opt.verbose.is_silent(), mesh_meta.len());
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

    let pb = utils::new_progress_bar(opt.verbose.is_silent(), mesh_meta.len());
    pb.set_message("Building Meshes");

    let load_config = LoadConfig {
        reverse: config.reverse,
        invert_tets: config.invert_tets,
    };

    let attrib_config = AttribConfig {
        attributes: &config.attributes,
        colors: &config.colors,
        texcoords: &config.texcoords,
        material_attribute: &config.material_attribute,
    };

    let process_attrib_error = |e| {
        pb.println(format!("{}: {}, Skipping...", style("WARNING").yellow(), e));
    };

    // Load all meshes with the appropriate conversions and attribute transfers.
    let meshes: Vec<_> = mesh_meta
        .into_par_iter()
        .progress_with(pb.clone())
        .filter_map(|(name, frame, path)| {
            load_and_clean_mesh(&path, load_config, attrib_config, process_attrib_error)
                .map(|(mesh, attrib_transfer)| (name, frame, mesh, attrib_transfer))
        })
        .collect();

    pb.finish_with_message("Done building meshes");

    if meshes.is_empty() {
        return Err(Error::NoMeshesFound);
    }

    let dt = if let Some(dt) = config.time_step {
        dt
    } else {
        1.0 / config.fps as f32
    };

    export::export_clean_meshes(
        meshes,
        export::ExportConfig {
            textures: config.textures,
            materials: config.materials,
            output: config.output,
            time_step: dt,
            insert_vanishing_frames: config.insert_vanishing_frames,
            animate_normals: !config.no_animated_normals,
            animate_tangents: !config.no_animated_tangents,
            quiet: opt.verbose.is_silent(),
        },
    );

    Ok(())
}
