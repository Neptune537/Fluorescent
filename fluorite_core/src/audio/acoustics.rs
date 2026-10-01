//! Acoustic Ray Tracing and Reverb Simulation.

use crate::spatial::{FlatBvh, Ray};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Acoustic material surface absorption coefficients across 3 frequency bands (Low, Mid, High).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AcousticMaterial {
    pub absorption: [f32; 3], // 0.0 = total reflection, 1.0 = total absorption
    pub scattering: f32,      // Surface roughness diffusion
}

impl AcousticMaterial {
    pub fn concrete() -> Self {
        Self {
            absorption: [0.01, 0.02, 0.03], // Highly reflective
            scattering: 0.1,
        }
    }

    pub fn wood() -> Self {
        Self {
            absorption: [0.15, 0.11, 0.10],
            scattering: 0.25,
        }
    }

    pub fn carpet_or_fabric() -> Self {
        Self {
            absorption: [0.08, 0.25, 0.50], // Highly absorbs high frequencies
            scattering: 0.6,
        }
    }
}

/// 3D Spatial Audio Emitter.
#[derive(Debug, Clone)]
pub struct SoundEmitter {
    pub id: u32,
    pub position: Vec3,
    pub base_gain: f32,
    pub inner_radius: f32,
    pub max_radius: f32,
}

/// 3D Spatial Audio Listener (Player Head/Ears).
#[derive(Debug, Clone)]
pub struct SoundListener {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

/// Dynamic acoustic ray.
#[derive(Debug, Clone)]
pub struct AcousticRay {
    pub origin: Vec3,
    pub direction: Vec3,
    pub energy: [f32; 3],
    pub distance_traveled: f32,
    pub bounce_count: u32,
}

/// Acoustic Simulation Output for a listener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticResult {
    /// Occlusion factor: 0.0 = clear line of sight, 1.0 = completely obstructed
    pub occlusion_factor: f32,
    /// Estimated Room Reverb Decay Time $RT_{60}$ in seconds
    pub rt60_seconds: f32,
    /// Early reflection delays in milliseconds
    pub early_reflection_delays_ms: Vec<f32>,
    /// High-frequency low-pass cutoff filter (Hz) based on air absorption and wall transmission
    pub low_pass_cutoff_hz: f32,
}

/// Real-Time Acoustic Tracer.
pub struct AcousticTracer {
    pub ray_count: usize,
    pub max_bounces: u32,
}

impl AcousticTracer {
    pub fn new(ray_count: usize, max_bounces: u32) -> Self {
        Self {
            ray_count,
            max_bounces,
        }
    }

    /// Traces sound propagation between emitter and listener through geometry.
    pub fn trace(
        &self,
        emitter: &SoundEmitter,
        listener: &SoundListener,
        bvh: &FlatBvh,
        room_volume_m3: f32,
        room_surface_area_m2: f32,
        average_absorption: f32,
    ) -> AcousticResult {
        let direct_vec = listener.position - emitter.position;
        let direct_dist = direct_vec.length();

        // 1. Direct Occlusion Raycast
        let direct_ray = Ray::new(
            emitter.position.into(),
            direct_vec.normalize_or_zero().into(),
            0.0,
            direct_dist,
        );
        let occlusion_hit = bvh.raycast(&direct_ray, &[]);

        let is_occluded = match occlusion_hit {
            Some(h) => h.distance < direct_dist - 0.1,
            None => false,
        };

        let occlusion_factor = if is_occluded { 0.85 } else { 0.0 };
        let low_pass = if is_occluded { 1200.0 } else { 20000.0 };

        // 2. Sabine's Reverberation Equation: $RT_{60} = 0.161 \cdot \frac{V}{S \cdot \bar{\alpha}}$
        let total_absorption_area = (room_surface_area_m2 * average_absorption).max(1.0);
        let rt60 = (0.161 * room_volume_m3 / total_absorption_area).clamp(0.1, 8.0);

        // 3. Early reflections calculation
        let speed_of_sound_m_s = 343.0;
        let mut reflection_delays = Vec::new();
        let direct_time_ms = (direct_dist / speed_of_sound_m_s) * 1000.0;

        // Trace a set of spherical acoustic rays
        let step = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt()); // golden angle
        for i in 0..self.ray_count {
            let y = 1.0 - (i as f32 / (self.ray_count - 1).max(1) as f32) * 2.0;
            let radius = (1.0 - y * y).max(0.0).sqrt();
            let theta = step * i as f32;

            let dir = Vec3::new(radius * theta.cos(), y, radius * theta.sin()).normalize();
            let ray = Ray::new(emitter.position.into(), dir.into(), 0.0, 50.0);

            if let Some(first_hit) = bvh.raycast(&ray, &[]) {
                let hit_pos = emitter.position + dir * first_hit.distance;
                let to_listener = listener.position - hit_pos;
                let bounce_dist = first_hit.distance + to_listener.length();
                let bounce_time_ms = (bounce_dist / speed_of_sound_m_s) * 1000.0;

                let delay = bounce_time_ms - direct_time_ms;
                if delay > 1.0 && delay < 100.0 {
                    reflection_delays.push(delay);
                }
            }
        }

        AcousticResult {
            occlusion_factor,
            rt60_seconds: rt60,
            early_reflection_delays_ms: reflection_delays,
            low_pass_cutoff_hz: low_pass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sabine_reverb_calculation() {
        let tracer = AcousticTracer::new(16, 2);
        let emitter = SoundEmitter {
            id: 1,
            position: Vec3::new(0.0, 1.0, 0.0),
            base_gain: 1.0,
            inner_radius: 1.0,
            max_radius: 50.0,
        };
        let listener = SoundListener {
            position: Vec3::new(5.0, 1.0, 5.0),
            forward: Vec3::Z,
            up: Vec3::Y,
        };
        let bvh = FlatBvh::default();

        let result = tracer.trace(&emitter, &listener, &bvh, 1000.0, 600.0, 0.15);
        assert!(result.rt60_seconds > 0.5);
        assert_eq!(result.occlusion_factor, 0.0);
    }
}
