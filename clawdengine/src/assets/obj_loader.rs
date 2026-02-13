use std::path::Path;

use anyhow::{Context, Result};

use crate::renderer::mesh::{Vertex, compute_tangents};

#[allow(dead_code)]
pub struct LoadedMesh {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[allow(dead_code)]
pub fn load_obj(path: impl AsRef<Path>) -> Result<Vec<LoadedMesh>> {
    let path = path.as_ref();
    let (models, _materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)
        .with_context(|| format!("Failed to load OBJ: {}", path.display()))?;

    let mut meshes = Vec::with_capacity(models.len());

    for model in models {
        let mesh = &model.mesh;
        let vertex_count = mesh.positions.len() / 3;
        let has_normals = !mesh.normals.is_empty();
        let has_uvs = !mesh.texcoords.is_empty();

        let mut vertices = Vec::with_capacity(vertex_count);

        for i in 0..vertex_count {
            let px = mesh.positions[i * 3];
            let py = mesh.positions[i * 3 + 1];
            let pz = mesh.positions[i * 3 + 2];

            let (nx, ny, nz) = if has_normals {
                (
                    mesh.normals[i * 3],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                )
            } else {
                (0.0, 1.0, 0.0)
            };

            let (u, v) = if has_uvs {
                (
                    mesh.texcoords[i * 2],
                    1.0 - mesh.texcoords[i * 2 + 1], // UV flip Y for wgpu
                )
            } else {
                (0.0, 0.0)
            };

            vertices.push(Vertex {
                position: [px, py, pz],
                normal: [nx, ny, nz],
                uv: [u, v],
                tangent: [0.0; 4],
            });
        }

        let indices_clone = mesh.indices.clone();
        compute_tangents(&mut vertices, &indices_clone);
        meshes.push(LoadedMesh {
            name: model.name,
            vertices,
            indices: indices_clone,
        });
    }

    Ok(meshes)
}
