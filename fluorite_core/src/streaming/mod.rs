//! Continuous Asynchronous World Streaming & Virtual Texturing Architecture.
//!
//! Streams gigabytes of geometry and texture assets directly from NVMe storage into VRAM
//! with zero loading screens across hundreds of square kilometers.

pub mod virtual_texture;
pub mod world_streamer;
pub mod pcg;

pub use virtual_texture::{FeedbackBuffer, PageCoord, PageTableEntry, VirtualTextureSystem};
pub use world_streamer::{
    ChunkCoord, ChunkState, StreamingChunk, StreamingConfig, StreamingPriority,
    WorldStreamingManager,
};
