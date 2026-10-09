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
        use noise::{NoiseFn, Perlin};
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;
        use crate::ecs::world::StreamingCommand;
        use crate::streaming::world_streamer::ChunkCoord;

        let chunk_size_meters = 256.0; // default from StreamingConfig
        let perlin = Perlin::new(self.seed as u32);

        // 1. Calculate world coordinates
        let world_x = (chunk_x as f32 + 0.5) * chunk_size_meters;
        let world_z = (chunk_z as f32 + 0.5) * chunk_size_meters;

        // 2. Sample noise for heightmap
        let mut heightmap = vec![0.0; 64 * 64];
        for i in 0..64 {
            for j in 0..64 {
                let px = world_x - (chunk_size_meters / 2.0) + (i as f32 / 64.0) * chunk_size_meters;
                let pz = world_z - (chunk_size_meters / 2.0) + (j as f32 / 64.0) * chunk_size_meters;

                let height = perlin.get([
                    px as f64 * config.biome_scale as f64,
                    pz as f64 * config.biome_scale as f64,
                ]) as f32 * 100.0;

                heightmap[i * 64 + j] = height;
            }
        }

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

        // 4. Use Poisson Disk Sampling or similar scatter algorithms
        let mut rng = StdRng::seed_from_u64(self.seed ^ (chunk_x as u64).wrapping_shl(32) ^ (chunk_z as u64));
        let area = chunk_size_meters * chunk_size_meters;
        let scatter_count = (area * config.density) as usize;
        let actual_count = rng.gen_range(scatter_count / 2..=scatter_count + (scatter_count / 2).max(1));

        if let Some(sender) = command_sender {
            let coord = ChunkCoord::new(chunk_x, chunk_z);
            let _ = sender.send(StreamingCommand::SpawnEntities(coord, actual_count));
        }
    }
}
