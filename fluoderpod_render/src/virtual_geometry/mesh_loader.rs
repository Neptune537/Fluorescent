use super::{ClusterData, ClusterHeader};

/// A dummy mesh loader/partitioner that chunks an index buffer into fixed-size meshlets (e.g. max 64 triangles).
/// In the future, this will be replaced by an offline asset pipeline tool.
pub struct GltfMeshLoader;

impl GltfMeshLoader {
    /// Simulates partitioning a mesh into rigid clusters of ~64 triangles.
    pub fn partition_mesh(
        vertices: &[[f32; 3]],
        indices: &[u32],
        max_triangles_per_cluster: u32,
    ) -> Vec<ClusterData> {
        let mut clusters = Vec::new();

        let max_indices = max_triangles_per_cluster * 3;
        let mut current_cluster_id = 0;

        for chunk in indices.chunks(max_indices as usize) {
            // Compute a simple bounding sphere for the cluster
            let mut center = [0.0, 0.0, 0.0];
            let mut count = 0;

            for &idx in chunk {
                if let Some(vertex) = vertices.get(idx as usize) {
                    center[0] += vertex[0];
                    center[1] += vertex[1];
                    center[2] += vertex[2];
                    count += 1;
                }
            }

            if count > 0 {
                center[0] /= count as f32;
                center[1] /= count as f32;
                center[2] /= count as f32;
            }

            let mut radius = 0.0_f32;
            for &idx in chunk {
                if let Some(vertex) = vertices.get(idx as usize) {
                    let dx = vertex[0] - center[0];
                    let dy = vertex[1] - center[1];
                    let dz = vertex[2] - center[2];
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    if dist > radius {
                        radius = dist;
                    }
                }
            }

            // Create dummy vertex data (just mapping the vertices used in this chunk for simplicity)
            // A real implementation would deduplicate and compress this data.
            let mut cluster_vertices = Vec::new();
            for &idx in chunk {
                if let Some(vertex) = vertices.get(idx as usize) {
                    cluster_vertices.extend_from_slice(bytemuck::cast_slice(&[*vertex]));
                }
            }

            let cluster_indices: Vec<u32> = (0..chunk.len() as u32).collect();
            let index_data = bytemuck::cast_slice(&cluster_indices).to_vec();

            let header = ClusterHeader {
                sphere_bounds: [center[0], center[1], center[2], radius],
                lod_error: 0.1, // Dummy error for highest LOD
                parent_lod_error: std::f32::MAX, // Root node has infinite parent error
                cluster_id: current_cluster_id,
                parent_cluster_id: 0, // No parent for now
                vertex_offset: 0, // Will be computed when appending to a global buffer
                vertex_count: chunk.len() as u32,
                index_offset: 0,
                index_count: chunk.len() as u32,
                flags: super::CLUSTER_FLAG_STATIC,
                dynamic_buffer_slot: 0,
                _pad: [0; 2],
            };

            clusters.push(ClusterData {
                header,
                vertex_data: cluster_vertices,
                index_data,
            });

            current_cluster_id += 1;
        }

        clusters
    }
}
