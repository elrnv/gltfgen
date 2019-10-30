mod export;

use regex::Regex;

use glob::glob;
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
    /// Use `#` to match a frame number. If more than one '#' is used, the first match will
    /// correspond to the frame number.
    #[structopt(parse(from_str))]
    pattern: String,

    /// Frames per second. 1.0/fps gives the time step between discrete frames.
    #[structopt(short, long)]
    fps: Option<usize>,

    /// Time step in seconds between discrete frames. If 'fps' is also provided, this parameter is
    /// ignored.
    #[structopt(short, long, default_value = "1.0")]
    time_step: f32,
}

fn glob_to_regex(glob: &str) -> Regex {
    let mut regex = String::from("^");

    // If we are doing extended matching, this boolean is true when we are inside
    // a group (eg {*.html,*.js}), and false otherwise.
    let mut in_group = false;

    let mut prev_c = None;
    let mut glob_iter = glob.chars().peekable();
    while let Some(c) = glob_iter.next() {
        match c {
            '#' => {
                // Special character indicating a frame number digit
                regex.push_str("([0-9]*)");
            }
            // Escape special characters
            '/' | '$' | '^' | '+' | '.' | '(' | ')' | '=' | '!' | '|' => {
                //regex.push_str("\\");
                regex.push(c);
            }

            '?' => {
                regex.push('.');
            }

            '[' | ']' => {
                regex.push(c);
            }

            '{' => {
                in_group = true;
                regex.push('(');
            }

            '}' => {
                in_group = false;
                regex.push(')');
            }

            ',' => {
                if in_group {
                    regex.push('|');
                } else {
                    regex.push_str("\\");
                    regex.push(c);
                }
            }

            '*' => {
                // Check if there are multiple consecutive ** in the pattern.
                let mut count = 1;
                while glob_iter.peek() == Some(&'*') {
                    count += 1;
                    glob_iter.next();
                }
                let next_c = glob_iter.peek();

                if count > 1
                    && (next_c == Some(&'/') || next_c.is_none())
                    && (prev_c == Some('/') || prev_c.is_none())
                {
                    // Multiple * detected
                    // match zero or more path segments
                    regex.push_str("?:[^/]*?:/|$*");
                    glob_iter.next(); // consume '/' if any.
                } else {
                    // Single * detected
                    regex.push_str("[^/]*"); // match one path segment
                }
            }

            _ => regex.push(c),
        }
        prev_c = Some(c);
    }

    regex.push('$');

    Regex::new(&regex).unwrap()
}

fn main() {
    let opt = Opt::from_args();
    let regex = glob_to_regex(&opt.pattern);
    let pattern = opt
        .pattern
        .replace("*#*", "*")
        .replace("*#", "*")
        .replace("#*", "*")
        .replace("#", "*");
    let mut meshes = Vec::new();
    for entry in glob(&pattern).expect("ERROR: Failed to read input glob pattern") {
        match entry {
            Ok(path) => {
                let caps = regex.captures(path.to_str().unwrap()).unwrap();
                let frame = caps[1]
                    .parse::<usize>()
                    .expect("ERROR: Failed to parse frame number");
                if let Ok(polymesh) = geo::io::load_polymesh::<f64, _>(&path) {
                    let TriMesh {
                        vertex_positions,
                        indices,
                        face_indices,
                        face_offsets,
                        vertex_attributes,
                        face_attributes,
                        face_vertex_attributes,
                        face_edge_attributes,
                    } = TriMesh::from(polymesh);
                    let meshf32 = TriMesh {
                        vertex_positions: geo::mesh::attrib::IntrinsicAttribute::from_vec(
                            vertex_positions
                                .iter()
                                .map(|&x| [x[0] as f32, x[1] as f32, x[2] as f32])
                                .collect(),
                        ),
                        indices,
                        face_indices,
                        face_offsets,
                        vertex_attributes,
                        face_attributes,
                        face_vertex_attributes,
                        face_edge_attributes,
                    };
                    meshes.push((frame, meshf32));
                } else if let Ok(polymesh) = geo::io::load_polymesh::<f32, _>(&path) {
                    meshes.push((frame, TriMesh::<f32>::from(polymesh)));
                } else if let Ok(tetmesh) = geo::io::load_tetmesh::<f64, _>(&path) {
                    let TriMesh {
                        vertex_positions,
                        indices,
                        face_indices,
                        face_offsets,
                        vertex_attributes,
                        face_attributes,
                        face_vertex_attributes,
                        face_edge_attributes,
                    } = tetmesh.surface_trimesh();
                    let meshf32 = TriMesh {
                        vertex_positions: geo::mesh::attrib::IntrinsicAttribute::from_vec(
                            vertex_positions
                                .iter()
                                .map(|&x| [x[0] as f32, x[1] as f32, x[2] as f32])
                                .collect(),
                        ),
                        indices,
                        face_indices,
                        face_offsets,
                        vertex_attributes,
                        face_attributes,
                        face_vertex_attributes,
                        face_edge_attributes,
                    };
                    meshes.push((frame, meshf32));
                } else if let Ok(tetmesh) = geo::io::load_tetmesh::<f32, _>(path) {
                    meshes.push((frame, tetmesh.surface_trimesh()));
                }
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
}
