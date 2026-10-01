//! Advanced Lighting Systems (Phase 3)
//!
//! Includes Dynamic Global Illumination (SSGI/RTGI) and Virtual Shadow Maps (VSM)
//! for scalable, high-resolution rendering.

pub mod vsm;
pub mod gi;

pub use vsm::{VirtualShadowMapSystem, VirtualShadowMapConfig};
pub use gi::{DynamicGISystem, GIConfig};
