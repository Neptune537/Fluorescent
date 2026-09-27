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
        let drag_magnitude = 0.5 * fluid_density * speed * speed * drag_coefficient * cross_sectional_area;
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
}
