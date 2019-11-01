use geo::mesh::TriMesh;
use regex::Regex;

pub fn trimesh_f64_to_f32(mesh: TriMesh<f64>) -> TriMesh<f32> {
    let TriMesh {
        vertex_positions,
        indices,
        vertex_attributes,
        face_attributes,
        face_vertex_attributes,
        face_edge_attributes,
    } = mesh;
    TriMesh {
        vertex_positions: geo::mesh::attrib::IntrinsicAttribute::from_vec(
            vertex_positions
                .iter()
                .map(|&x| [x[0] as f32, x[1] as f32, x[2] as f32])
                .collect(),
        ),
        indices,
        vertex_attributes,
        face_attributes,
        face_vertex_attributes,
        face_edge_attributes,
    }
}

pub fn glob_to_regex(glob: &str) -> Regex {
    let mut regex = String::from("^");

    let mut prev_c = None;
    let mut glob_iter = glob.chars().peekable();
    while let Some(c) = glob_iter.next() {
        match c {
            '#' => {
                // Special character indicating a frame number digit
                regex.push_str("(?P<frame>[0-9]+)");
            }
            // Escape special characters
            '$' | '^' | '+' | '.' | '(' | ')' | '=' | '!' | '|' => {
                regex.push_str("\\");
                regex.push(c);
            }
            '{' => regex.push('('),
            '}' => regex.push(')'),
            '?' => regex.push('.'),
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

    Regex::new(&regex).expect("ERROR: Failed to convert glob to regular expression")
}

/// Remove braces from the pattern.
pub fn remove_braces(pattern: &str) -> String {
    let mut out_pattern = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if chars.peek() == Some(&'{') || chars.peek() == Some(&'}') {
                out_pattern.push(chars.next().unwrap());
                continue;
            }
        }
        if c == '{' || c == '}' {
            continue;
        }

        out_pattern.push(c);
    }
    out_pattern
}
