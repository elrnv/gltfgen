use std::path::{Path, PathBuf};

use meshx::algo::Merge;
use rayon::prelude::*;

#[macro_use]
pub mod utils;
#[macro_use]
pub mod attrib;
pub mod config;
pub mod error;
pub mod export;
pub mod material;
pub mod mesh;
pub mod texture;

pub use attrib::*;
pub use error::*;
pub use material::*;
pub use texture::*;
pub use utils::*;

use mesh::{trimesh_f64_to_f32, Mesh};

/// Configuration for loading meshes.
#[derive(Clone, Copy, Debug)]
pub struct LoadConfig {
    pub reverse: bool,
    pub invert_tets: bool,
}

/// Configuration for locating attributes within loaded meshes.
#[derive(Clone, Copy, Debug)]
pub struct AttribConfig<'a> {
    pub attributes: &'a AttributeInfo,
    pub colors: &'a AttributeInfo,
    pub texcoords: &'a TextureAttributeInfo,
    pub material_attribute: &'a str,
}

/// Convenience routine for loading and meshes extracting the required
/// attributes and removing all extraneous attributes.
pub fn load_and_clean_meshes(
    mesh_meta: Vec<(String, u32, PathBuf)>,
    load_config: LoadConfig,
    attrib_config: AttribConfig,
) -> Vec<(String, u32, Mesh, AttribTransfer)> {
    let process_attrib_error = |e| log::warn!("{}, Skipping...", e);
    mesh_meta
        .into_par_iter()
        .filter_map(|(name, frame, path)| {
            load_and_clean_mesh(&path, load_config, attrib_config, process_attrib_error)
                .map(|(mesh, attrib_transfer)| (name, frame, mesh, attrib_transfer))
        })
        .collect()
}

/// Convenience routine just for extracting the required
/// attributes and removing all extraneous attributes from the given vector of named meshes.
///
/// The resulting vector can then be fed into `export_clean_meshes` for exporting.
/// The frame numbers are inferred from the order in which the meshes are given.
pub fn clean_named_meshes(
    meshes: Vec<(String, Mesh)>,
    attrib_config: AttribConfig,
) -> Vec<(String, u32, Mesh, AttribTransfer)> {
    let process_attrib_error = |e| log::warn!("{}, Skipping...", e);
    meshes
        .into_par_iter()
        .enumerate()
        .map(|(frame, (name, mut mesh))| {
            let attrib_transfer = clean_mesh(&mut mesh, attrib_config, process_attrib_error);
            (name, frame as u32, mesh, attrib_transfer)
        })
        .collect()
}

pub fn load_and_clean_mesh(
    path: &Path,
    load_config: LoadConfig,
    attrib_config: AttribConfig,
    process_attrib_error: impl FnMut(attrib::AttribError),
) -> Option<(Mesh, AttribTransfer)> {
    let mut mesh = load_mesh(path, load_config)?;
    let attrib_transfer = clean_mesh(&mut mesh, attrib_config, process_attrib_error);
    Some((mesh, attrib_transfer))
}

pub fn load_mesh(path: impl AsRef<Path>, config: LoadConfig) -> Option<Mesh> {
    load_mesh_impl(path.as_ref(), config)
}

fn load_mesh_impl(path: &Path, config: LoadConfig) -> Option<Mesh> {
    let polymesh_tris = if let Ok(polymesh) = meshx::io::load_polymesh::<f64, _>(path) {
        trimesh_f64_to_f32(meshx::TriMesh::from(polymesh))
    } else if let Ok(polymesh) = meshx::io::load_polymesh::<f32, _>(path) {
        meshx::TriMesh::<f32>::from(polymesh)
    } else {
        meshx::TriMesh::default()
    };

    let polymesh_tris = mesh::remove_orphaned_vertices(polymesh_tris);

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
    Some(mesh)
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

        let load_config = LoadConfig {
            reverse: false,
            invert_tets: false,
        };

        let attrib_config = AttribConfig {
            attributes: &attributes,
            colors: &colors,
            texcoords: &texcoords,
            material_attribute,
        };

        let meshes = load_and_clean_meshes(mesh_meta, load_config, attrib_config);

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

        let load_config = LoadConfig {
            reverse: false,
            invert_tets: false,
        };

        let attrib_config = AttribConfig {
            attributes: &attributes,
            colors: &colors,
            texcoords: &texcoords,
            material_attribute,
        };

        let meshes = load_and_clean_meshes(mesh_meta, load_config, attrib_config);

        assert!(!meshes.is_empty());
    }

    #[test]
    fn multi() {
        let mut mesh_meta = Vec::new();

        mesh_meta.extend((1..=12).map(|frame| {
            let path = format!("./assets/box_rotate_{}.vtk", frame);
            (String::from("box_rotate"), frame, PathBuf::from(path))
        }));
        mesh_meta.extend((1..=2).map(|frame| {
            let path = format!("./assets/tet_{}.vtk", frame);
            (String::from("tet"), frame, PathBuf::from(path))
        }));
        mesh_meta.extend((1..=1).map(|frame| {
            let path = "./assets/box_triangulated.vtk";
            (String::from("box_triangulated"), frame, PathBuf::from(path))
        }));

        let attributes = "{\"pressure\": f32}".parse().unwrap();
        let colors = AttributeInfo::default();
        let texcoords = TextureAttributeInfo::default();
        let material_attribute = "mtl_id";

        let load_config = LoadConfig {
            reverse: true,
            invert_tets: false,
        };

        let attrib_config = AttribConfig {
            attributes: &attributes,
            colors: &colors,
            texcoords: &texcoords,
            material_attribute,
        };

        let meshes = load_and_clean_meshes(mesh_meta, load_config, attrib_config);

        assert!(!meshes.is_empty());

        let dt = 1.0 / 24.0;

        let artifact = "./tests/artifacts/multi_test.glb";

        export::export_clean_meshes(
            meshes,
            export::ExportConfig {
                textures: Vec::new(),
                materials: Vec::new(),
                output: artifact.into(),
                time_step: dt,
                insert_vanishing_frames: false,
                animate_normals: false,
                animate_tangents: false,
                quiet: true,
            },
        );

        let actual = Gltf::open(artifact).unwrap().blob;

        dbg!(&actual);
    }

    // Alternative way to export loaded meshes.
    #[test]
    fn multi_alt() {
        let load_config = LoadConfig {
            reverse: true,
            invert_tets: false,
        };

        // Meshes can be loaded without any kind of attribute processing.
        let meshes = vec![(
            "box_triangulated".to_owned(),
            load_mesh("./assets/box_triangulated.vtk", load_config).unwrap(),
        )];

        assert!(!meshes.is_empty());

        let dt = 1.0 / 24.0;

        let artifact = "./tests/artifacts/multi_alt_test.glb";

        let attrib_config = AttribConfig {
            attributes: &"{\"pressure\": f32}".parse().unwrap(),
            colors: &AttributeInfo::default(),
            texcoords: &TextureAttributeInfo::default(),
            material_attribute: "mtl_id",
        };

        // The loaded meshes are then processed according to the given AttribConfig.
        export::export_named_meshes(
            meshes,
            attrib_config,
            export::ExportConfig {
                textures: Vec::new(),
                materials: Vec::new(),
                output: artifact.into(),
                time_step: dt,
                insert_vanishing_frames: false,
                animate_normals: false,
                animate_tangents: false,
                quiet: true,
            },
        );

        let actual = Gltf::open(artifact).unwrap().blob;

        dbg!(&actual);
    }
}
