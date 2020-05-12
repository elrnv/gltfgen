use std::path::PathBuf;

#[macro_use]
pub mod attrib;
pub mod export;
pub mod material;
pub mod mesh;
pub mod texture;
pub mod utils;

pub use attrib::*;
pub use material::*;
pub use texture::*;
pub use utils::*;

use mesh::Mesh;

use rayon::prelude::*;

pub struct LoadConfig<'a> {
    pub attributes: &'a AttributeInfo,
    pub colors: &'a AttributeInfo,
    pub texcoords: &'a TextureAttributeInfo,
    pub material_attribute: &'a str,
    pub reverse: bool,
    pub invert_tets: bool,
}

pub fn load_meshes<P: Fn() + Send + Sync>(
    mesh_meta: Vec<(String, usize, PathBuf)>,
    config: LoadConfig,
    progress_inc: P,
) -> Vec<(String, usize, Mesh, AttribTransfer)> {
    mesh_meta
        .into_par_iter()
        .filter_map(|(name, frame, path)| {
            progress_inc();
            let mut mesh = if let Ok(polymesh) = gut::io::load_polymesh::<f64, _>(&path) {
                polymesh.into()
            } else if let Ok(polymesh) = gut::io::load_polymesh::<f32, _>(&path) {
                polymesh.into()
            } else if let Ok(tetmesh) = gut::io::load_tetmesh::<f64, _>(&path) {
                let mut mesh = Mesh::from(tetmesh);
                if config.invert_tets {
                    mesh.reverse();
                }
                mesh
            } else if let Ok(tetmesh) = gut::io::load_tetmesh::<f32, _>(&path) {
                let mut mesh = Mesh::from(tetmesh);
                if config.invert_tets {
                    mesh.reverse();
                }
                mesh
            } else if let Ok(ptcloud) = gut::io::load_pointcloud::<f64, _>(&path) {
                ptcloud.into()
            } else if let Ok(ptcloud) = gut::io::load_pointcloud::<f32, _>(&path) {
                ptcloud.into()
            } else {
                return None;
            };

            if config.reverse {
                mesh.reverse();
            }

            let attrib_transfer = clean_attributes(
                &mut mesh,
                config.attributes,
                config.colors,
                config.texcoords,
                config.material_attribute,
            );

            Some((name, frame, mesh, attrib_transfer))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_rotate_obj() {
        //{
        //    use std::io::{Read, BufRead};
        //    use std::io::BufReader;
        //    let f1 = std::fs::File::open("./assets/box_rotate_1.obj").unwrap();
        //    let f2 = std::fs::File::open("./assets/box_rotate_2.obj").unwrap();
        //    for line in BufReader::new(f1).lines() {
        //        dbg!(line);
        //    }
        //    for line in BufReader::new(f2).lines() {
        //        dbg!(line);
        //    }
        //}

        let mesh_meta: Vec<_> = (1..=24)
            .map(|frame| {
                let path = format!("./assets/box_rotate_{}.obj", frame);
                (String::from("box_rotate"), frame, PathBuf::from(path))
            })
            .collect();
        //dbg!(&mesh_meta);

        let attributes = AttributeInfo::default();
        let colors = AttributeInfo::default();
        let texcoords: TextureAttributeInfo = "{\"uv\": f32}".parse().unwrap();
        let material_attribute = "mtl_id";

        let config = LoadConfig {
            attributes: &attributes,
            colors: &colors,
            texcoords: &texcoords,
            material_attribute,
            reverse: false,
            invert_tets: false,
        };

        load_meshes(mesh_meta, config, || {});
    }
}
