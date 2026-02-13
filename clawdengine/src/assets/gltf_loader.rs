use std::path::Path;

use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};

use crate::renderer::mesh::{compute_tangents, Vertex};

// ---- Output structures ----

pub struct GltfScene {
    pub meshes: Vec<GltfLoadedMesh>,
    pub textures: Vec<GltfLoadedTexture>,
}

pub struct GltfLoadedMesh {
    pub store_name: String,
    pub display_name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub transform: Mat4,
    pub material: Option<GltfLoadedMaterial>,
}

pub struct GltfLoadedMaterial {
    pub albedo: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub emission: Vec3,
    pub albedo_texture: Option<usize>,
    pub normal_texture: Option<usize>,
}

pub struct GltfLoadedTexture {
    pub cache_key: String,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

// ---- Public API ----

pub fn load_gltf(path: impl AsRef<Path>) -> Result<GltfScene> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    let (document, buffers, images) =
        gltf::import(path).with_context(|| format!("Failed to import glTF: {}", path_str))?;

    let textures = extract_textures(&images, &path_str);

    let mut scene = GltfScene {
        meshes: Vec::new(),
        textures,
    };

    // Build a texture index map: gltf texture index → our textures vec index
    let tex_index_map: Vec<Option<usize>> = document
        .textures()
        .map(|tex| {
            let img_idx = tex.source().index();
            if img_idx < scene.textures.len() {
                Some(img_idx)
            } else {
                None
            }
        })
        .collect();

    // Walk the scene graph (flatten hierarchy)
    for gltf_scene in document.scenes() {
        for node in gltf_scene.nodes() {
            visit_node(&node, Mat4::IDENTITY, &buffers, &tex_index_map, &path_str, &mut scene);
        }
    }

    log::info!(
        "Loaded glTF: {} ({} meshes, {} textures)",
        path_str,
        scene.meshes.len(),
        scene.textures.len()
    );

    Ok(scene)
}

// ---- Node traversal ----

fn visit_node(
    node: &gltf::Node,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    tex_index_map: &[Option<usize>],
    file_path: &str,
    scene: &mut GltfScene,
) {
    let local_transform = node_transform(node);
    let world_transform = parent_transform * local_transform;

    if let Some(mesh) = node.mesh() {
        let node_name = node
            .name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("node_{}", node.index()));

        for (prim_idx, primitive) in mesh.primitives().enumerate() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                log::warn!(
                    "Skipping non-triangle primitive in {}/{} (mode: {:?})",
                    node_name,
                    prim_idx,
                    primitive.mode()
                );
                continue;
            }

            if let Some((mut vertices, indices)) = extract_primitive(&primitive, buffers) {
                // Compute tangents if the glTF didn't provide them
                let has_tangents = vertices.iter().any(|v| {
                    v.tangent[0] != 0.0 || v.tangent[1] != 0.0 || v.tangent[2] != 0.0
                });
                if !has_tangents {
                    compute_tangents(&mut vertices, &indices);
                }

                let material = convert_material(&primitive.material(), tex_index_map);

                let store_name = format!("gltf:{}#{}/{}", file_path, node_name, prim_idx);
                let display_name = if mesh.primitives().len() > 1 {
                    format!("{}.{}", node_name, prim_idx)
                } else {
                    node_name.clone()
                };

                scene.meshes.push(GltfLoadedMesh {
                    store_name,
                    display_name,
                    vertices,
                    indices,
                    transform: world_transform,
                    material: Some(material),
                });
            }
        }
    }

    // Recurse into children
    for child in node.children() {
        visit_node(&child, world_transform, buffers, tex_index_map, file_path, scene);
    }
}

fn node_transform(node: &gltf::Node) -> Mat4 {
    let (translation, rotation, scale) = node.transform().decomposed();
    let t = Vec3::from(translation);
    let r = Quat::from_array(rotation);
    let s = Vec3::from(scale);
    Mat4::from_scale_rotation_translation(s, r, t)
}

// ---- Primitive extraction ----

fn extract_primitive(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Option<(Vec<Vertex>, Vec<u32>)> {
    let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    let vertex_count = positions.len();

    let normals: Vec<[f32; 3]> = reader
        .read_normals()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; vertex_count]);

    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|tc| tc.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; vertex_count]);

    let tangents: Vec<[f32; 4]> = reader
        .read_tangents()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 0.0, 0.0, 1.0]; vertex_count]);

    let indices: Vec<u32> = reader
        .read_indices()
        .map(|idx| idx.into_u32().collect())
        .unwrap_or_else(|| (0..vertex_count as u32).collect());

    let vertices: Vec<Vertex> = (0..vertex_count)
        .map(|i| Vertex {
            position: positions[i],
            normal: normals[i],
            uv: uvs[i],
            tangent: tangents[i],
        })
        .collect();

    Some((vertices, indices))
}

// ---- Material conversion ----

fn convert_material(
    gltf_mat: &gltf::Material,
    tex_index_map: &[Option<usize>],
) -> GltfLoadedMaterial {
    let pbr = gltf_mat.pbr_metallic_roughness();
    let base = pbr.base_color_factor();
    // Only use emissive_factor if there's no emissive texture
    // (our shader doesn't support emissive textures, so [1,1,1] would blow out the image)
    let emission = if gltf_mat.emissive_texture().is_some() {
        Vec3::ZERO
    } else {
        Vec3::from(gltf_mat.emissive_factor())
    };

    let albedo_texture = pbr
        .base_color_texture()
        .and_then(|info| tex_index_map.get(info.texture().index()).copied().flatten());

    let normal_texture = gltf_mat
        .normal_texture()
        .and_then(|info| tex_index_map.get(info.texture().index()).copied().flatten());

    GltfLoadedMaterial {
        albedo: Vec3::new(base[0], base[1], base[2]),
        roughness: pbr.roughness_factor(),
        metallic: pbr.metallic_factor(),
        emission,
        albedo_texture,
        normal_texture,
    }
}

// ---- Texture extraction ----

fn extract_textures(images: &[gltf::image::Data], file_path: &str) -> Vec<GltfLoadedTexture> {
    images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            log::info!(
                "glTF texture {}: {}x{}, format={:?}, {} bytes",
                i, img.width, img.height, img.format, img.pixels.len()
            );
            let rgba = match img.format {
                gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
                gltf::image::Format::R8G8B8 => rgb_to_rgba(&img.pixels),
                gltf::image::Format::R16G16B16A16 => rgba16_to_rgba8(&img.pixels),
                gltf::image::Format::R16G16B16 => rgb16_to_rgba8(&img.pixels),
                gltf::image::Format::R8 => r8_to_rgba(&img.pixels),
                gltf::image::Format::R16 => r16_to_rgba(&img.pixels),
                gltf::image::Format::R8G8 => rg8_to_rgba(&img.pixels),
                gltf::image::Format::R16G16 => rg16_to_rgba(&img.pixels),
                gltf::image::Format::R32G32B32FLOAT => rgb32f_to_rgba(&img.pixels),
                gltf::image::Format::R32G32B32A32FLOAT => rgba32f_to_rgba(&img.pixels),
            };

            GltfLoadedTexture {
                cache_key: format!("gltf-embedded:{}#texture_{}", file_path, i),
                rgba,
                width: img.width,
                height: img.height,
            }
        })
        .collect()
}

// ---- Format conversion helpers ----

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let pixel_count = rgb.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.extend_from_slice(chunk);
        rgba.push(255);
    }
    rgba
}

fn rgba16_to_rgba8(data: &[u8]) -> Vec<u8> {
    data.chunks_exact(2)
        .map(|c| (u16::from_le_bytes([c[0], c[1]]) >> 8) as u8)
        .collect()
}

fn rgb16_to_rgba8(data: &[u8]) -> Vec<u8> {
    let pixel_count = data.len() / 6;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in data.chunks_exact(6) {
        for ch in pixel.chunks_exact(2) {
            rgba.push((u16::from_le_bytes([ch[0], ch[1]]) >> 8) as u8);
        }
        rgba.push(255);
    }
    rgba
}

fn r8_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 4);
    for &v in data {
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    rgba
}

fn r16_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 2);
    for chunk in data.chunks_exact(2) {
        let v = (u16::from_le_bytes([chunk[0], chunk[1]]) >> 8) as u8;
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    rgba
}

fn rg8_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 2);
    for chunk in data.chunks_exact(2) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], 0, 255]);
    }
    rgba
}

fn rg16_to_rgba(data: &[u8]) -> Vec<u8> {
    let pixel_count = data.len() / 4;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in data.chunks_exact(4) {
        let r = (u16::from_le_bytes([pixel[0], pixel[1]]) >> 8) as u8;
        let g = (u16::from_le_bytes([pixel[2], pixel[3]]) >> 8) as u8;
        rgba.extend_from_slice(&[r, g, 0, 255]);
    }
    rgba
}

fn rgb32f_to_rgba(data: &[u8]) -> Vec<u8> {
    let pixel_count = data.len() / 12;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in data.chunks_exact(12) {
        for ch in pixel.chunks_exact(4) {
            let f = f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]);
            rgba.push((f.clamp(0.0, 1.0) * 255.0) as u8);
        }
        rgba.push(255);
    }
    rgba
}

fn rgba32f_to_rgba(data: &[u8]) -> Vec<u8> {
    data.chunks_exact(4)
        .map(|ch| {
            let f = f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]);
            (f.clamp(0.0, 1.0) * 255.0) as u8
        })
        .collect()
}
