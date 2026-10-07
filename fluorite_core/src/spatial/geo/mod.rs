pub mod altitude_transition;
pub mod celestial;
pub mod coords;

pub use altitude_transition::{
    AltitudeTransitionManager, TransitionState, ViewportScaleMode,
    ALTITUDE_HYSTERESIS_METERS, ALTITUDE_TRANSITION_THRESHOLD_METERS,
};
pub use celestial::{
    compute_celestial_placements, compute_sky_illumination, horizontal_to_camera_vector,
    horizontal_to_enu_vector, CelestialBodyPlacement, CelestialType, SkyIllumination,
};
pub use coords::{ecef_to_enu, lat_lon_alt_to_ecef, lat_lon_alt_to_enu};
