pub mod camera;
pub mod cluster;
pub mod csm;
pub mod mesh;
pub mod pbr;
pub mod renderer;
pub mod rtgi;
pub mod shadow;
pub mod sss;
pub mod volumetrics;
pub mod vulkan_surface;

pub use camera::{Camera3D, ProjectionType};
pub use cluster::{
    ClusterAabb, ClusterCell, ClusterLightGrid, ClusterRecord, ClusteredLightOutput, GpuLight,
    PointLight, SpotLight, NUM_CLUSTERS_X, NUM_CLUSTERS_Y, NUM_CLUSTERS_Z, TOTAL_CLUSTERS,
};
pub use csm::{CascadeData, CascadedShadowMap, CsmConfig, NUM_CASCADES};
pub use mesh::{GltfMeshLoader, Model3D, SubMesh, Vertex3D};
pub use pbr::{
    distribution_ggx, evaluate_cook_torrance_brdf, fresnel_schlick,
    visibility_smith_ggx_correlated, CameraUniforms, PbrMaterialUniforms,
    MATERIAL_FLAG_ALPHA_BLEND, MATERIAL_FLAG_HAS_ALBEDO_MAP, MATERIAL_FLAG_HAS_EMISSIVE_MAP,
    MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_MAP, MATERIAL_FLAG_HAS_NORMAL_MAP,
    MATERIAL_FLAG_HAS_OCCLUSION_MAP,
};
pub use renderer::{QualityTier, Renderer};
pub use rtgi::{IrradianceProbe, RtgiConfig, RtgiPipeline, TlasInstance};
pub use shadow::{
    calculate_shadow_bias, compute_directional_shadow_matrices, compute_frustum_bounding_sphere,
    compute_frustum_corners, DirectionalLight, DirectionalShadowOutput, ShadowMapConfig,
    ShadowUniforms,
};
pub use sss::SubsurfaceProfile;
pub use volumetrics::{
    beer_lambert, henyey_greenstein_phase, VolumetricAtmosphereConfig, WaterCausticsConfig,
};
pub use vulkan_surface::{
    SurfaceConfig, SurfaceFormat, SwapchainStatus, VulkanSurfaceEmbedder, WindowHandleType,
};
