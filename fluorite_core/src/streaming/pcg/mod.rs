//! Procedural Content Generation (PCG) (Phase 3)
//!
//! PCG frameworks for terrain generation, vegetation placement, and biome mapping.

pub struct PcgSystem {
    pub seed: u64,
}

pub struct PcgConfig {
    pub biome_scale: f32,
    pub density: f32,
}

impl PcgSystem {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn generate_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        config: &PcgConfig,
        command_sender: Option<std::sync::mpsc::Sender<crate::ecs::world::StreamingCommand>>,
    ) {
        use noise::{NoiseFn, Perlin, Fbm, MultiFractal};
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;
        use crate::ecs::world::StreamingCommand;
        use crate::streaming::world_streamer::ChunkCoord;

        let chunk_size_meters = 256.0; // default from StreamingConfig

        // Setup FBM for terrain heightmap (multi-octave for realistic terrain)
        let fbm_terrain = Fbm::<Perlin>::new(self.seed as u32)
            .set_octaves(4)
            .set_frequency(1.0);

        // Setup a secondary noise for biome mapping (e.g. moisture/temperature)
        let fbm_biome = Fbm::<Perlin>::new(self.seed.wrapping_add(1) as u32)
            .set_octaves(2)
            .set_frequency(0.5);

        // 1. Calculate world coordinates
        let world_x = (chunk_x as f32 + 0.5) * chunk_size_meters;
        let world_z = (chunk_z as f32 + 0.5) * chunk_size_meters;

        // 2. Sample noise for heightmap and determine dominant biome per chunk area
        let mut heightmap = vec![0.0; 64 * 64];
        let mut chunk_biome_val = 0.0;
        for i in 0..64 {
            for j in 0..64 {
                let px = world_x - (chunk_size_meters / 2.0) + (i as f32 / 64.0) * chunk_size_meters;
                let pz = world_z - (chunk_size_meters / 2.0) + (j as f32 / 64.0) * chunk_size_meters;

                let nx = px as f64 * config.biome_scale as f64;
                let nz = pz as f64 * config.biome_scale as f64;

                let height = fbm_terrain.get([nx, nz]) as f32 * 100.0;
                heightmap[i * 64 + j] = height;

                if i == 32 && j == 32 {
                    chunk_biome_val = fbm_biome.get([nx, nz]) as f32;
                }
            }
        }

        let biome_density_multiplier = if chunk_biome_val > 0.3 {
            1.5 // Forest/Dense biome
        } else if chunk_biome_val < -0.3 {
            0.2 // Desert/Sparse biome
        } else {
            1.0 // Normal plains
        };

        // 3. Generate terrain mesh geometry based on the heightmap
        let mut _mesh_vertices = Vec::with_capacity(64 * 64);
        for i in 0..64 {
            for j in 0..64 {
                let px = world_x - (chunk_size_meters / 2.0) + (i as f32 / 64.0) * chunk_size_meters;
                let pz = world_z - (chunk_size_meters / 2.0) + (j as f32 / 64.0) * chunk_size_meters;
                let py = heightmap[i * 64 + j];
                _mesh_vertices.push([px, py, pz]);
            }
        }

        // 4. Poisson Disk Sampling approximation (Jittered grid)
        let mut rng = StdRng::seed_from_u64(self.seed ^ (chunk_x as u64).wrapping_shl(32) ^ (chunk_z as u64));
        let base_density = config.density * biome_density_multiplier;

        let mut actual_count = 0;

        if base_density > 0.0 {
            let cell_size = 1.0 / base_density.sqrt().max(0.001); // Approx distance between entities
            let cells_per_axis = (chunk_size_meters / cell_size).floor() as i32;

            for cx in 0..cells_per_axis {
                for cz in 0..cells_per_axis {
                    // Jitter randomly within the cell
                    let jitter_x: f32 = rng.gen_range(0.0..1.0);
                    let jitter_z: f32 = rng.gen_range(0.0..1.0);

                    let _pos_x = world_x - (chunk_size_meters / 2.0) + (cx as f32 + jitter_x) * cell_size;
                    let _pos_z = world_z - (chunk_size_meters / 2.0) + (cz as f32 + jitter_z) * cell_size;

                    actual_count += 1;
                }
            }
        }

        if let Some(sender) = command_sender {
            let coord = ChunkCoord::new(chunk_x, chunk_z);
            let _ = sender.send(StreamingCommand::SpawnEntities(coord, actual_count));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_chunk() {
        let system = PcgSystem::new(12345);
        let config = PcgConfig {
            biome_scale: 0.01,
            density: 0.05,
        };

        let (tx, rx) = std::sync::mpsc::channel();
        system.generate_chunk(0, 0, &config, Some(tx));

        let cmd = rx.recv().unwrap();
        match cmd {
            crate::ecs::world::StreamingCommand::SpawnEntities(coord, count) => {
                assert_eq!(coord.x, 0);
                assert_eq!(coord.z, 0);
                assert!(count > 0);
            }
            _ => panic!("Expected SpawnEntities command"),
        }
    }
}
