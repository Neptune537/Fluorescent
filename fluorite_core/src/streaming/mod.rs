//! Continuous Asynchronous World Streaming & Virtual Texturing Architecture.
//!
//! Streams gigabytes of geometry and texture assets directly from NVMe storage into VRAM
//! with zero loading screens across hundreds of square kilometers.

pub mod pcg;
pub mod sphere_quadtree;
pub mod virtual_texture;
pub mod world_streamer;

pub use sphere_quadtree::{
    generate_procedural_sphere_tile, EcefBoundingSphere, SphereNodeState, SphereQuadNode,
    SphereQuadtreeConfig, SphereQuadtreeManager, SphereTileData, SphereTileKey,
    MAX_SPHERE_QUADTREE_LOD, SPHERE_TILE_GRID_SIZE,
};
pub use virtual_texture::{FeedbackBuffer, PageCoord, PageTableEntry, VirtualTextureSystem};
pub use world_streamer::{
    ChunkCoord, ChunkState, StreamingChunk, StreamingConfig, StreamingPriority,
    WorldStreamingManager,
};
