use gltf::json;
use meshx::mesh::vertex_positions::VertexPositions;
use meshx::mesh::{PointCloud, PolyMesh, TetMesh, TriMesh};
use meshx::topology::NumVertices;

use crate::{AttribTransfer, MaterialIds};

/// Supported output mesh types.
#[derive(Debug)]
pub enum Mesh {
    TriMesh(Box<TriMesh<f32>>),
    PointCloud(PointCloud<f32>),
}

impl Mesh {
    pub fn is_empty(&self) -> bool {
        match self {
            Mesh::TriMesh(trimesh) => {
                trimesh.indices.is_empty() && trimesh.vertex_positions.is_empty()
            }
            Mesh::PointCloud(ptcloud) => ptcloud.vertex_positions.is_empty(),
        }
    }

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

    pub fn build_topology(
        &self,
        attrib_transfer: &AttribTransfer,
        data: &mut Vec<u8>,
        buffer_views: &mut Vec<json::buffer::View>,
        accessors: &mut Vec<json::Accessor>,
    ) -> (&[[f32; 3]], Option<Vec<json::Index<json::Accessor>>>) {
        match self {
            Mesh::TriMesh(ref trimesh) => (
                trimesh.vertex_positions.as_slice(),
                Some(build_indices(
                    &**trimesh,
                    attrib_transfer,
                    data,
                    buffer_views,
                    accessors,
                )),
            ),
            Mesh::PointCloud(PointCloud {
                vertex_positions, ..
            }) => (vertex_positions.as_slice(), None),
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
impl From<TriMesh<f32>> for Mesh {
    fn from(mesh: TriMesh<f32>) -> Self {
        Mesh::TriMesh(Box::new(mesh))
    }
}

impl From<PolyMesh<f32>> for Mesh {
    fn from(mesh: PolyMesh<f32>) -> Self {
        Mesh::from(TriMesh::<f32>::from(mesh))
    }
}

impl From<PolyMesh<f64>> for Mesh {
    fn from(mesh: PolyMesh<f64>) -> Self {
        Mesh::from(trimesh_f64_to_f32(TriMesh::from(mesh)))
    }
}

impl From<TetMesh<f32>> for Mesh {
    fn from(mesh: TetMesh<f32>) -> Self {
        Mesh::from(mesh.surface_trimesh())
    }
}

impl From<TetMesh<f64>> for Mesh {
    fn from(mesh: TetMesh<f64>) -> Self {
        Mesh::from(trimesh_f64_to_f32(mesh.surface_trimesh()))
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
        vertex_positions: meshx::attrib::IntrinsicAttribute::from_vec(
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
        vertex_positions: meshx::attrib::IntrinsicAttribute::from_vec(
            vertex_positions
                .iter()
                .map(|&x| [x[0] as f32, x[1] as f32, x[2] as f32])
                .collect(),
        ),
        vertex_attributes,
    }
}

fn push_indices(
    trimesh_indices: &[[usize; 3]],
    face_indices: impl ExactSizeIterator<Item = usize>,
    data: &mut Vec<u8>,
    buffer_views: &mut Vec<json::buffer::View>,
    accessors: &mut Vec<json::Accessor>,
    indices: &mut Vec<json::Index<json::Accessor>>,
) {
    use crate::export::{AccessorBuilder, BufferViewBuilder};
    use byteorder::{WriteBytesExt, LE};
    use num_traits::ToPrimitive;

    // Push indices to data buffer.
    let num_indices = face_indices.len() * 3;
    let byte_length = num_indices * std::mem::size_of::<u32>();
    let indices_view = json::buffer::View::new(byte_length, data.len())
        .with_target(json::buffer::Target::ElementArrayBuffer);

    let mut max_index = 0;
    let mut min_index = u32::MAX;
    for idx in face_indices {
        for &i in trimesh_indices[idx].iter() {
            let vidx = i
                .to_u32()
                .expect("Vertex index does not fit into a 32 bit unsigned integer.");
            max_index = max_index.max(vidx);
            min_index = min_index.min(vidx);
            data.write_u32::<LE>(vidx).unwrap();
        }
    }

    let idx_acc = json::Accessor::new(num_indices, json::accessor::ComponentType::U32)
        .with_buffer_view(buffer_views.len())
        .with_min_max(&[min_index][..], &[max_index][..]);

    buffer_views.push(indices_view);
    let idx_acc_index = accessors.len() as u32;
    accessors.push(idx_acc);
    indices.push(json::Index::new(idx_acc_index));
}

fn build_indices(
    trimesh: &TriMesh<f32>,
    attrib_transfer: &AttribTransfer,
    data: &mut Vec<u8>,
    buffer_views: &mut Vec<json::buffer::View>,
    accessors: &mut Vec<json::Accessor>,
) -> Vec<json::Index<json::Accessor>> {
    // Sort indices by associated materials (if any).
    let mut indices = Vec::new();

    match &attrib_transfer.material_ids {
        Some(MaterialIds::Local { .. }) => {
            unreachable!("All local material IDs should be converted to global at this point.");
        }
        Some(MaterialIds::Global { map }) => {
            // Each face has a unique material id, split indices into sections corresponding to the same material id.
            for face_indices in map.values() {
                push_indices(
                    trimesh.indices.as_slice(),
                    face_indices.iter().cloned(),
                    data,
                    buffer_views,
                    accessors,
                    &mut indices,
                );
            }
        }
        None => {
            // No materials to deal with, just push all the indices as they appear.
            push_indices(
                trimesh.indices.as_slice(),
                0..trimesh.indices.len(),
                data,
                buffer_views,
                accessors,
                &mut indices,
            );
        }
    }
    indices
}
