use gut::mesh::topology::NumVertices;
use gut::mesh::vertex_positions::VertexPositions;
use gut::mesh::{PointCloud, PolyMesh, TetMesh, TriMesh};

/// Supported output mesh types.
#[derive(Debug)]
pub enum Mesh {
    TriMesh(TriMesh<f32>),
    PointCloud(PointCloud<f32>),
}

impl Mesh {
    pub fn reverse(&mut self) {
        if let Mesh::TriMesh(mesh) = self {
            mesh.reverse();
        } /* else: Nothing to reverse */
    }

    /// Returns true if the `other` mesh has equivalent topology to `self`.
    pub fn eq_topo(&self, other: &Mesh) -> bool {
        match self {
            Mesh::TriMesh(self_mesh) => {
                if let Mesh::TriMesh(other_mesh) = other {
                    self_mesh.num_vertices() == other_mesh.num_vertices()
                        && self_mesh.indices == other_mesh.indices
                } else {
                    false
                }
            }
            Mesh::PointCloud(self_pts) => {
                if let Mesh::PointCloud(other_pts) = other {
                    self_pts.num_vertices() == other_pts.num_vertices()
                } else {
                    false
                }
            }
        }
    }
}

impl VertexPositions for Mesh {
    type Element = [f32; 3];

    fn vertex_positions(&self) -> &[Self::Element] {
        match self {
            Mesh::TriMesh(mesh) => mesh.vertex_positions(),
            Mesh::PointCloud(mesh) => mesh.vertex_positions(),
        }
    }
    fn vertex_positions_mut(&mut self) -> &mut [Self::Element] {
        match self {
            Mesh::TriMesh(mesh) => mesh.vertex_positions_mut(),
            Mesh::PointCloud(mesh) => mesh.vertex_positions_mut(),
        }
    }
}

impl From<PolyMesh<f32>> for Mesh {
    fn from(mesh: PolyMesh<f32>) -> Self {
        Mesh::TriMesh(TriMesh::<f32>::from(mesh))
    }
}

impl From<PolyMesh<f64>> for Mesh {
    fn from(mesh: PolyMesh<f64>) -> Self {
        Mesh::TriMesh(trimesh_f64_to_f32(TriMesh::from(mesh)))
    }
}

impl From<TetMesh<f32>> for Mesh {
    fn from(mesh: TetMesh<f32>) -> Self {
        Mesh::TriMesh(mesh.surface_trimesh())
    }
}

impl From<TetMesh<f64>> for Mesh {
    fn from(mesh: TetMesh<f64>) -> Self {
        Mesh::TriMesh(trimesh_f64_to_f32(mesh.surface_trimesh()))
    }
}

impl From<PointCloud<f32>> for Mesh {
    fn from(mesh: PointCloud<f32>) -> Self {
        Mesh::PointCloud(mesh)
    }
}

impl From<PointCloud<f64>> for Mesh {
    fn from(mesh: PointCloud<f64>) -> Self {
        Mesh::PointCloud(pointcloud_f64_to_f32(mesh))
    }
}

pub fn trimesh_f64_to_f32(mesh: TriMesh<f64>) -> TriMesh<f32> {
    let TriMesh {
        vertex_positions,
        indices,
        vertex_attributes,
        face_attributes,
        face_vertex_attributes,
        face_edge_attributes,
        attribute_value_cache,
    } = mesh;
    TriMesh {
        vertex_positions: gut::mesh::attrib::IntrinsicAttribute::from_vec(
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
        attribute_value_cache,
    }
}

pub fn pointcloud_f64_to_f32(ptcloud: PointCloud<f64>) -> PointCloud<f32> {
    let PointCloud {
        vertex_positions,
        vertex_attributes,
    } = ptcloud;
    PointCloud {
        vertex_positions: gut::mesh::attrib::IntrinsicAttribute::from_vec(
            vertex_positions
                .iter()
                .map(|&x| [x[0] as f32, x[1] as f32, x[2] as f32])
                .collect(),
        ),
        vertex_attributes,
    }
}
