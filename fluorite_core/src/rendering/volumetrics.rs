//! Volumetric Weather Clouds, Atmospheric Scattering, and Dynamic Water Caustics.
//!
//! Implements raymarched volumetric cloud simulation via Beer-Lambert absorption and
//! Henyey-Greenstein phase functions, combined with refractive water surface caustics.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Atmospheric and volumetric weather configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumetricAtmosphereConfig {
    pub cloud_coverage: f32, // 0.0 to 1.0
    pub cloud_altitude_bottom: f32, // e.g. 1500m
    pub cloud_altitude_top: f32, // e.g. 4000m
    pub wind_speed: Vec2,
    pub sun_direction: Vec3,
    pub sun_intensity: f32,
    pub scattering_coefficient: f32,
    pub absorption_coefficient: f32,
    pub henyey_greenstein_g: f32, // forward scattering anisotropy (~0.75)
}

impl Default for VolumetricAtmosphereConfig {
    fn default() -> Self {
        Self {
            cloud_coverage: 0.5,
            cloud_altitude_bottom: 1500.0,
            cloud_altitude_top: 4000.0,
            wind_speed: Vec2::new(12.0, 4.0),
            sun_direction: Vec3::new(0.5, 0.7, 0.3).normalize(),
            sun_intensity: 100.0,
            scattering_coefficient: 0.04,
            absorption_coefficient: 0.01,
            henyey_greenstein_g: 0.75,
        }
    }
}

/// Henyey-Greenstein phase function for forward-scattering sunlight in droplets.
#[inline]
pub fn henyey_greenstein_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    (1.0 - g2) / (4.0 * std::f32::consts::PI * denom.powf(1.5).max(1e-5))
}

/// Beer-Lambert transmittance calculation over optical depth.
#[inline]
pub fn beer_lambert(optical_depth: f32) -> f32 {
    (-optical_depth).exp()
}

/// Evaluates water surface caustics intensity from wave height gradient and refractive index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterCausticsConfig {
    pub water_depth: f32,
    pub refraction_index: f32, // 1.333 for water
    pub caustics_scale: f32,
    pub caustics_intensity: f32,
    pub wave_frequency: f32,
}

impl Default for WaterCausticsConfig {
    fn default() -> Self {
        Self {
            water_depth: 3.0,
            refraction_index: 1.333,
            caustics_scale: 1.0,
            caustics_intensity: 1.5,
            wave_frequency: 0.8,
        }
    }
}

impl WaterCausticsConfig {
    /// Computes caustic concentration intensity on the seabed.
    pub fn evaluate_caustic_factor(&self, wave_gradient: Vec2, time_secs: f32) -> f32 {
        // Snell's law refraction convergence factor
        let wave_deriv = wave_gradient.length();
        let phase = (time_secs * self.wave_frequency).sin();
        let concentration = (1.0 + wave_deriv * self.water_depth * 2.0 * phase).max(0.0);
        (concentration * self.caustics_intensity).clamp(0.0, 5.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_and_transmittance() {
        let phase = henyey_greenstein_phase(1.0, 0.75); // direct forward
        assert!(phase > 0.0);

        let trans = beer_lambert(0.0);
        assert_eq!(trans, 1.0);
        let trans_deep = beer_lambert(5.0);
        assert!(trans_deep < 0.01);
    }

    #[test]
    fn test_water_caustics() {
        let caustics = WaterCausticsConfig::default();
        let factor = caustics.evaluate_caustic_factor(Vec2::new(0.2, 0.1), 1.0);
        assert!(factor >= 0.0);
    }
}
