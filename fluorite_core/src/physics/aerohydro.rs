//! Aerodynamics and Hydrodynamic Water Buoyancy Physics.
//!
//! Provides Archimedes principle-based fluid displacement, water drag,
//! and quadratic aerodynamic drag and lift forces.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Fluid medium properties (Air or Water).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidMedium {
    pub density_kg_m3: f32, // 1.225 for Air, 1000.0 for Fresh Water, 1025.0 for Sea Water
    pub linear_viscosity: f32,
}

impl FluidMedium {
    pub fn air() -> Self {
        Self {
            density_kg_m3: 1.225,
            linear_viscosity: 0.018,
        }
    }

    pub fn water() -> Self {
        Self {
            density_kg_m3: 1000.0,
            linear_viscosity: 1.0,
        }
    }
}

/// Buoyancy and hydrodynamic forces calculator.
pub struct Hydrodynamics;

impl Hydrodynamics {
    /// Computes Archimedes buoyant force: $F_{buoyancy} = \rho \cdot V_{submerged} \cdot g$.
    pub fn compute_buoyant_force(
        submerged_volume_m3: f32,
        water_density: f32,
        gravity: f32,
    ) -> Vec3 {
        let upward_force = water_density * submerged_volume_m3 * gravity;
        Vec3::new(0.0, upward_force, 0.0)
    }

    /// Computes quadratic fluid drag force: $F_{drag} = -\frac{1}{2} \cdot \rho \cdot v^2 \cdot C_d \cdot A \cdot \hat{v}$.
    pub fn compute_fluid_drag(
        velocity: Vec3,
        fluid_density: f32,
        drag_coefficient: f32,
        cross_sectional_area: f32,
    ) -> Vec3 {
        let speed = velocity.length();
        if speed < 1e-4 {
            return Vec3::ZERO;
        }

        let dir = velocity / speed;
        let drag_magnitude =
            0.5 * fluid_density * speed * speed * drag_coefficient * cross_sectional_area;
        -dir * drag_magnitude
    }

    /// Computes aerodynamic lift force perpendicular to velocity:
    /// $F_{lift} = \frac{1}{2} \cdot \rho \cdot v^2 \cdot C_l \cdot A \cdot \hat{n}_{lift}$.
    pub fn compute_aerodynamic_lift(
        velocity: Vec3,
        lift_direction: Vec3,
        air_density: f32,
        lift_coefficient: f32,
        wing_area: f32,
    ) -> Vec3 {
        let speed = velocity.length();
        if speed < 1e-4 {
            return Vec3::ZERO;
        }

        let lift_magnitude = 0.5 * air_density * speed * speed * lift_coefficient * wing_area;
        lift_direction.normalize_or_zero() * lift_magnitude
    }

    /// Evaluates analytic Gerstner ocean surface wave displacement at world position (x, z) and time t.
    /// This math is mirrored directly in the GPU vertex shader for 100% physics/visual sync.
    pub fn evaluate_gerstner_wave(
        pos_xz: glam::Vec2,
        time_secs: f32,
        waves: &[GerstnerWave],
    ) -> (f32, glam::Vec3) {
        let mut wave_height: f32 = 0.0;
        let mut normal = glam::Vec3::new(0.0, 1.0, 0.0);

        for wave in waves {
            let dir = wave.direction.normalize_or_zero();
            let phase = wave.frequency * dir.dot(pos_xz) + time_secs * wave.phase_speed;
            let sin_p = phase.sin();
            let cos_p = phase.cos();

            // Height displacement
            wave_height += wave.amplitude * sin_p;

            // Analytic normal derivation
            let wa = wave.frequency * wave.amplitude;
            normal.x -= dir.x * wa * cos_p;
            normal.z -= dir.y * wa * cos_p;
        }

        (wave_height, normal.normalize_or_zero())
    }

    /// Computes multi-point buoyant equilibrium and torque for a boat hull on dynamic water.
    pub fn evaluate_hull_at_waves(
        probes: &[Vec3],
        boat_position: Vec3,
        boat_velocity: Vec3,
        time_secs: f32,
        waves: &[GerstnerWave],
        water_density: f32,
        probe_volume: f32,
        hull_drag_coeff: f32,
    ) -> (Vec3, Vec3) {
        let mut total_force = Vec3::ZERO;
        let mut total_torque = Vec3::ZERO;

        for &probe_offset in probes {
            let world_probe = boat_position + probe_offset;
            let (wave_y, _normal) = Self::evaluate_gerstner_wave(
                glam::Vec2::new(world_probe.x, world_probe.z),
                time_secs,
                waves,
            );

            // Submerged depth
            let depth = wave_y - world_probe.y;
            if depth > 0.0 {
                let submerged_factor = (depth / 1.0).clamp(0.0, 1.0);
                let local_volume = probe_volume * submerged_factor;
                let b_force = Self::compute_buoyant_force(local_volume, water_density, 9.81);
                let drag = Self::compute_fluid_drag(boat_velocity, water_density, hull_drag_coeff, 0.5);

                let probe_total_force = b_force + drag * (1.0 / probes.len() as f32);
                total_force += probe_total_force;
                total_torque += probe_offset.cross(probe_total_force);
            }
        }

        (total_force, total_torque)
    }
}

/// Parameters for an analytic Gerstner ocean wave.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GerstnerWave {
    pub direction: glam::Vec2,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase_speed: f32,
    pub steepness: f32,
}

impl Default for GerstnerWave {
    fn default() -> Self {
        Self {
            direction: glam::Vec2::new(1.0, 0.0),
            amplitude: 0.5,
            frequency: 0.2,
            phase_speed: 1.5,
            steepness: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buoyancy() {
        let force = Hydrodynamics::compute_buoyant_force(1.0, 1000.0, 9.81);
        assert!((force.y - 9810.0).abs() < 1e-2);
    }

    #[test]
    fn test_drag() {
        let vel = Vec3::new(10.0, 0.0, 0.0);
        let drag = Hydrodynamics::compute_fluid_drag(vel, 1.225, 0.3, 2.0);
        assert!(drag.x < 0.0); // opposes forward motion
    }

    #[test]
    fn test_gerstner_wave_and_boat_hull() {
        let waves = [GerstnerWave::default()];
        let (height, normal) = Hydrodynamics::evaluate_gerstner_wave(glam::Vec2::new(0.0, 0.0), 0.0, &waves);
        assert_eq!(height, 0.0);
        assert!(normal.y > 0.0);

        // 4 hull probes (bow, stern, port, starboard)
        let probes = [
            Vec3::new(0.0, -0.5, 2.0),
            Vec3::new(0.0, -0.5, -2.0),
            Vec3::new(1.0, -0.5, 0.0),
            Vec3::new(-1.0, -0.5, 0.0),
        ];
        let boat_pos = Vec3::new(0.0, 0.0, 0.0);
        let boat_vel = Vec3::new(0.0, 0.0, 5.0);

        let (force, torque) = Hydrodynamics::evaluate_hull_at_waves(
            &probes,
            boat_pos,
            boat_vel,
            0.5,
            &waves,
            1000.0,
            0.5,
            0.3,
        );

        assert!(force.y > 0.0, "Upward buoyancy should be generated on submerged hull");
        assert!(torque.is_finite(), "Torque calculation must remain stable");
    }
}
