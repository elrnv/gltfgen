use std::path::{Path, PathBuf};

#[macro_use]
pub mod utils;
#[macro_use]
pub mod attrib;
pub mod export;
pub mod material;
pub mod mesh;
pub mod texture;

pub use attrib::*;
pub use material::*;
pub use texture::*;
pub use utils::*;

use mesh::Mesh;

use rayon::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct LoadConfig<'a> {
    pub attributes: &'a AttributeInfo,
    pub colors: &'a AttributeInfo,
    pub texcoords: &'a TextureAttributeInfo,
    pub material_attribute: &'a str,
    pub reverse: bool,
    pub invert_tets: bool,
}

pub fn load_meshes(
    mesh_meta: Vec<(String, usize, PathBuf)>,
    config: LoadConfig,
) -> Vec<(String, usize, Mesh, AttribTransfer)> {
    let process_attrib_error = |e| log::warn!("{}, Skipping...", e);
    mesh_meta
        .into_par_iter()
        .filter_map(|(name, frame, path)| {
            load_mesh(&path, config, process_attrib_error)
                .map(|(mesh, attrib_transfer)| (name, frame, mesh, attrib_transfer))
        })
        .collect()
}

pub fn load_mesh(
    path: &Path,
    config: LoadConfig,
    process_attrib_error: impl FnMut(attrib::AttribError),
) -> Option<(Mesh, AttribTransfer)> {
    let mut mesh = if let Ok(polymesh) = meshx::io::load_polymesh::<f64, _>(path) {
        polymesh.into()
    } else if let Ok(polymesh) = meshx::io::load_polymesh::<f32, _>(path) {
        polymesh.into()
    } else if let Ok(tetmesh) = meshx::io::load_tetmesh::<f64, _>(path) {
        let mut mesh = Mesh::from(tetmesh);
        if config.invert_tets {
            mesh.reverse();
        }
        mesh
    } else if let Ok(tetmesh) = meshx::io::load_tetmesh::<f32, _>(path) {
        let mut mesh = Mesh::from(tetmesh);
        if config.invert_tets {
            mesh.reverse();
        }
        mesh
    } else if let Ok(ptcloud) = meshx::io::load_pointcloud::<f64, _>(path) {
        ptcloud.into()
    } else if let Ok(ptcloud) = meshx::io::load_pointcloud::<f32, _>(path) {
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
        process_attrib_error,
    );

    Some((mesh, attrib_transfer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_rotate_obj() {
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

        load_meshes(mesh_meta, config);
    }
}
