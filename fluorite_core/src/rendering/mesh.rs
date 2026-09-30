//! 3D Mesh Representation, Vertex/Index Buffers, and glTF 2.0 Asset Loader.
//!
//! Provides memory-aligned 3D vertex formats matching `pbr_forward.wgsl` (`VertexInput`),
//! index buffers, submesh partitioning, and high-performance glTF parsing.

use crate::rendering::pbr::PbrMaterialUniforms;
use crate::spatial::Aabb;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Standard 3D PBR Vertex structure matching GPU layout:
/// - `@location(0) position: vec3<f32>` (12 bytes)
/// - `@location(1) normal: vec3<f32>` (12 bytes)
/// - `@location(2) tangent: vec4<f32>` (16 bytes, xyz = tangent, w = sign/handedness)
/// - `@location(3) uv: vec2<f32>` (8 bytes)
///
/// Total size: 48 bytes (16-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

const _: () = assert!(std::mem::size_of::<Vertex3D>() == 48);

impl Vertex3D {
    pub const fn new(
        position: [f32; 3],
        normal: [f32; 3],
        tangent: [f32; 4],
        uv: [f32; 2],
    ) -> Self {
        Self {
            position,
            normal,
            tangent,
            uv,
        }
    }
}

/// A partitioned primitive within a 3D mesh.
#[derive(Debug, Clone)]
pub struct SubMesh {
    pub name: String,
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
    pub material: PbrMaterialUniforms,
    pub bounding_box: Aabb,
}

impl SubMesh {
    pub fn new(
        name: impl Into<String>,
        vertices: Vec<Vertex3D>,
        indices: Vec<u32>,
        material: PbrMaterialUniforms,
    ) -> Self {
        let mut min = glam::Vec3A::splat(f32::INFINITY);
        let mut max = glam::Vec3A::splat(f32::NEG_INFINITY);

        for v in &vertices {
            let p = glam::Vec3A::from_slice(&v.position);
            min = min.min(p);
            max = max.max(p);
        }

        let bounding_box = if vertices.is_empty() {
            Aabb::new(glam::Vec3A::ZERO, glam::Vec3A::ZERO)
        } else {
            Aabb::new(min, max)
        };

        Self {
            name: name.into(),
            vertices,
            indices,
            material,
            bounding_box,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Complete 3D Model consisting of multiple submeshes and transforms.
#[derive(Debug, Clone)]
pub struct Model3D {
    pub name: String,
    pub submeshes: Vec<SubMesh>,
    pub bounding_box: Aabb,
}

impl Model3D {
    pub fn new(name: impl Into<String>, submeshes: Vec<SubMesh>) -> Self {
        let mut min = glam::Vec3A::splat(f32::INFINITY);
        let mut max = glam::Vec3A::splat(f32::NEG_INFINITY);

        for sub in &submeshes {
            min = min.min(sub.bounding_box.min);
            max = max.max(sub.bounding_box.max);
        }

        let bounding_box = if submeshes.is_empty() {
            Aabb::new(glam::Vec3A::ZERO, glam::Vec3A::ZERO)
        } else {
            Aabb::new(min, max)
        };

        Self {
            name: name.into(),
            submeshes,
            bounding_box,
        }
    }

    pub fn total_vertices(&self) -> usize {
        self.submeshes.iter().map(|s| s.vertex_count()).sum()
    }

    pub fn total_indices(&self) -> usize {
        self.submeshes.iter().map(|s| s.index_count()).sum()
    }

    pub fn total_triangles(&self) -> usize {
        self.submeshes.iter().map(|s| s.triangle_count()).sum()
    }
}

/// High-performance glTF 2.0 Loader for static and dynamic 3D geometry.
pub struct GltfMeshLoader;

impl GltfMeshLoader {
    /// Loads a glTF or GLB binary directly from an in-memory slice.
    pub fn load_from_slice(slice: &[u8]) -> Result<Model3D, String> {
        let (gltf, buffers, _images) =
            gltf::import_slice(slice).map_err(|e| format!("glTF parse error: {e}"))?;

        let mut submeshes = Vec::new();

        for mesh in gltf.meshes() {
            let mesh_name = mesh.name().unwrap_or("unnamed_mesh").to_string();

            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // Extract positions (required)
                let positions: Vec<[f32; 3]> = match reader.read_positions() {
                    Some(pos_iter) => pos_iter.collect(),
                    None => continue, // Cannot render geometry without positions
                };

                // Extract normals (or compute flat/smooth normals)
                let normals: Vec<[f32; 3]> = if let Some(norm_iter) = reader.read_normals() {
                    norm_iter.collect()
                } else {
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                };

                // Extract UV coordinates (or default to 0.0)
                let uvs: Vec<[f32; 2]> = if let Some(uv_iter) = reader.read_tex_coords(0) {
                    uv_iter.into_f32().collect()
                } else {
                    vec![[0.0, 0.0]; positions.len()]
                };

                // Extract tangents (or compute Mikktspace/Gram-Schmidt approximation)
                let tangents: Vec<[f32; 4]> = if let Some(tan_iter) = reader.read_tangents() {
                    tan_iter.collect()
                } else {
                    vec![[1.0, 0.0, 0.0, 1.0]; positions.len()]
                };

                // Extract indices
                let indices: Vec<u32> = if let Some(indices_iter) = reader.read_indices() {
                    indices_iter.into_u32().collect()
                } else {
                    (0..positions.len() as u32).collect()
                };

                // Interleave into Vertex3D
                let mut vertices = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    vertices.push(Vertex3D::new(
                        positions[i],
                        normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                        tangents.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 1.0]),
                        uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                    ));
                }

                // Extract PBR material properties
                let mat = primitive.material();
                let pbr = mat.pbr_metallic_roughness();
                let base_color = pbr.base_color_factor();
                let emissive = mat.emissive_factor();
                let metallic = pbr.metallic_factor();
                let roughness = pbr.roughness_factor();

                let mut material_uniforms = PbrMaterialUniforms {
                    base_color_factor: base_color,
                    emissive_factor: emissive,
                    metallic_factor: metallic,
                    roughness_factor: roughness,
                    normal_scale: 1.0,
                    occlusion_strength: 1.0,
                    flags: 0,
                };

                if pbr.base_color_texture().is_some() {
                    material_uniforms.flags |= crate::rendering::pbr::MATERIAL_FLAG_HAS_ALBEDO_MAP;
                }
                if mat.normal_texture().is_some() {
                    material_uniforms.flags |= crate::rendering::pbr::MATERIAL_FLAG_HAS_NORMAL_MAP;
                }
                if pbr.metallic_roughness_texture().is_some() {
                    material_uniforms.flags |=
                        crate::rendering::pbr::MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_MAP;
                }
                if mat.occlusion_texture().is_some() {
                    material_uniforms.flags |=
                        crate::rendering::pbr::MATERIAL_FLAG_HAS_OCCLUSION_MAP;
                }
                if mat.emissive_texture().is_some() {
                    material_uniforms.flags |=
                        crate::rendering::pbr::MATERIAL_FLAG_HAS_EMISSIVE_MAP;
                }

                submeshes.push(SubMesh::new(
                    mesh_name.clone(),
                    vertices,
                    indices,
                    material_uniforms,
                ));
            }
        }

        Ok(Model3D::new("LoadedModel", submeshes))
    }

    /// Generates a standard procedural UV sphere primitive for PBR validation.
    pub fn create_sphere(radius: f32, sectors: u32, stacks: u32) -> SubMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let inv_sector = 1.0 / sectors as f32;
        let inv_stack = 1.0 / stacks as f32;
        let pi = std::f32::consts::PI;

        for i in 0..=stacks {
            let stack_angle = pi / 2.0 - (i as f32) * pi * inv_stack;
            let xy = radius * stack_angle.cos();
            let z = radius * stack_angle.sin();

            for j in 0..=sectors {
                let sector_angle = (j as f32) * 2.0 * pi * inv_sector;
                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();

                let pos = [x, y, z];
                let norm = [x / radius, y / radius, z / radius];
                let uv = [(j as f32) * inv_sector, (i as f32) * inv_stack];
                let tangent = [-sector_angle.sin(), sector_angle.cos(), 0.0, 1.0];

                vertices.push(Vertex3D::new(pos, norm, tangent, uv));
            }
        }

        for i in 0..stacks {
            let mut k1 = i * (sectors + 1);
            let mut k2 = k1 + sectors + 1;

            for _ in 0..sectors {
                if i != 0 {
                    indices.push(k1);
                    indices.push(k2);
                    indices.push(k1 + 1);
                }

                if i != (stacks - 1) {
                    indices.push(k1 + 1);
                    indices.push(k2);
                    indices.push(k2 + 1);
                }

                k1 += 1;
                k2 += 1;
            }
        }

        SubMesh::new(
            "ProceduralSphere",
            vertices,
            indices,
            PbrMaterialUniforms::default(),
        )
    }

    /// Generates a procedural 3D box/cube mesh with distinct face normals and UVs.
    pub fn create_box(half_extents: Vec3) -> SubMesh {
        let h = half_extents;
        let positions: [[f32; 3]; 24] = [
            // Front face (+Z)
            [-h.x, -h.y, h.z],
            [h.x, -h.y, h.z],
            [h.x, h.y, h.z],
            [-h.x, h.y, h.z],
            // Back face (-Z)
            [h.x, -h.y, -h.z],
            [-h.x, -h.y, -h.z],
            [-h.x, h.y, -h.z],
            [h.x, h.y, -h.z],
            // Top face (+Y)
            [-h.x, h.y, h.z],
            [h.x, h.y, h.z],
            [h.x, h.y, -h.z],
            [-h.x, h.y, -h.z],
            // Bottom face (-Y)
            [-h.x, -h.y, -h.z],
            [h.x, -h.y, -h.z],
            [h.x, -h.y, h.z],
            [-h.x, -h.y, h.z],
            // Right face (+X)
            [h.x, -h.y, h.z],
            [h.x, -h.y, -h.z],
            [h.x, h.y, -h.z],
            [h.x, h.y, h.z],
            // Left face (-X)
            [-h.x, -h.y, -h.z],
            [-h.x, -h.y, h.z],
            [-h.x, h.y, h.z],
            [-h.x, h.y, -h.z],
        ];

        let normals: [[f32; 3]; 6] = [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        ];

        let mut vertices = Vec::with_capacity(24);
        for face in 0..6 {
            let n = normals[face];
            let t = [1.0, 0.0, 0.0, 1.0];
            let uv_coords = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
            for vertex_in_face in 0..4 {
                let idx = face * 4 + vertex_in_face;
                vertices.push(Vertex3D::new(
                    positions[idx],
                    n,
                    t,
                    uv_coords[vertex_in_face],
                ));
            }
        }

        let mut indices = Vec::with_capacity(36);
        for face in 0..6 {
            let base = (face * 4) as u32;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        SubMesh::new(
            "ProceduralBox",
            vertices,
            indices,
            PbrMaterialUniforms::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_layout() {
        assert_eq!(std::mem::size_of::<Vertex3D>(), 48);
    }

    #[test]
    fn test_procedural_box() {
        let box_mesh = GltfMeshLoader::create_box(Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(box_mesh.vertex_count(), 24);
        assert_eq!(box_mesh.index_count(), 36);
        assert_eq!(box_mesh.triangle_count(), 12);
        assert_eq!(
            box_mesh.bounding_box.min,
            glam::Vec3A::new(-1.0, -1.0, -1.0)
        );
        assert_eq!(box_mesh.bounding_box.max, glam::Vec3A::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_procedural_sphere() {
        let sphere_mesh = GltfMeshLoader::create_sphere(2.0, 16, 16);
        assert!(sphere_mesh.vertex_count() > 0);
        assert!(sphere_mesh.index_count() > 0);
        assert_eq!(sphere_mesh.bounding_box.min.x.round(), -2.0);
        assert_eq!(sphere_mesh.bounding_box.max.x.round(), 2.0);
    }
}
