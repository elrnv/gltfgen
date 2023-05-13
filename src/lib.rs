use std::path::{Path, PathBuf};

use meshx::algo::Merge;
use rayon::prelude::*;

#[macro_use]
pub mod utils;
#[macro_use]
pub mod attrib;
pub mod config;
pub mod export;
pub mod material;
pub mod mesh;
pub mod texture;

pub use attrib::*;
pub use material::*;
pub use texture::*;
pub use utils::*;

use mesh::{trimesh_f64_to_f32, Mesh};

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
    let polymesh_tris = if let Ok(polymesh) = meshx::io::load_polymesh::<f64, _>(path) {
        trimesh_f64_to_f32(meshx::TriMesh::from(polymesh))
    } else if let Ok(polymesh) = meshx::io::load_polymesh::<f32, _>(path) {
        meshx::TriMesh::<f32>::from(polymesh)
    } else {
        meshx::TriMesh::default()
    };

    let mut tetmesh_tris = if let Ok(tetmesh) = meshx::io::load_tetmesh::<f64, _>(path) {
        trimesh_f64_to_f32(tetmesh.surface_trimesh())
    } else if let Ok(tetmesh) = meshx::io::load_tetmesh::<f32, _>(path) {
        tetmesh.surface_trimesh()
    } else {
        meshx::TriMesh::default()
    };

    // Reverse triangles that came from tets. This is faster than actually inverting tets but
    // achieves the same result.
    if config.invert_tets {
        tetmesh_tris.reverse();
    }

    tetmesh_tris.merge(polymesh_tris);
    let mut mesh = Mesh::from(tetmesh_tris);

    if mesh.is_empty() {
        mesh = if let Ok(ptcloud) = meshx::io::load_pointcloud::<f64, _>(path) {
            ptcloud.into()
        } else if let Ok(ptcloud) = meshx::io::load_pointcloud::<f32, _>(path) {
            ptcloud.into()
        } else {
            return None;
        };
    }

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
    use gltf::Gltf;

    #[test]
    fn box_rotate_obj() {
        let mesh_meta: Vec<_> = (1..=12)
            .map(|frame| {
                let path = format!("./assets/box_rotate_{}.obj", frame);
                (String::from("box_rotate"), frame, PathBuf::from(path))
            })
            .collect();

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

        let meshes = load_meshes(mesh_meta, config);

        assert!(!meshes.is_empty());
    }

    #[test]
    fn box_triangulated() {
        let mesh_meta: Vec<_> = (1..=1)
            .map(|frame| {
                let path = "./assets/box_triangulated.vtk";
                (String::from("box_triangulated"), frame, PathBuf::from(path))
            })
            .collect();

        let attributes = AttributeInfo::default();
        let colors = AttributeInfo::default();
        let texcoords = TextureAttributeInfo::default();
        let material_attribute = "mtl_id";

        let config = LoadConfig {
            attributes: &attributes,
            colors: &colors,
            texcoords: &texcoords,
            material_attribute,
            reverse: false,
            invert_tets: false,
        };

        let meshes = load_meshes(mesh_meta, config);

        assert!(!meshes.is_empty());
    }

    #[test]
    fn multi() {
        let mut mesh_meta = Vec::new();

        // mesh_meta.extend((1..=12)
        //     .map(|frame| {
        //         let path = format!("./assets/box_rotate_{}.vtk", frame);
        //         (String::from("box_rotate"), frame, PathBuf::from(path))
        //     }));
        // mesh_meta.extend((1..=2)
        //     .map(|frame| {
        //         let path = format!("./assets/tet_{}.vtk", frame);
        //         (String::from("tet"), frame, PathBuf::from(path))
        //     }));
        mesh_meta.extend((1..=1).map(|frame| {
            let path = "./assets/box_triangulated.vtk";
            (String::from("box_triangulated"), frame, PathBuf::from(path))
        }));

        let attributes = "{\"pressure\": f32}".parse().unwrap();
        let colors = AttributeInfo::default();
        let texcoords = TextureAttributeInfo::default();
        let material_attribute = "mtl_id";

        let config = LoadConfig {
            attributes: &attributes,
            colors: &colors,
            texcoords: &texcoords,
            material_attribute,
            reverse: true,
            invert_tets: false,
        };

        let meshes = load_meshes(mesh_meta, config);

        assert!(!meshes.is_empty());

        let dt = 1.0 / 24.0;

        let artifact = "./tests/artifacts/multi_test.glb";

        export::export(meshes, artifact.into(), dt, true, Vec::new(), Vec::new());

        let actual = Gltf::open(artifact).unwrap().blob;

        dbg!(&actual);
    }
}
