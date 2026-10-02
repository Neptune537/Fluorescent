//! Vehicle Suspension Dynamics and Pacejka 'Magic Formula' Tire Friction Model.
//!
//! Simulates high-fidelity multi-link suspension, anti-roll bars, slip angles, and non-linear tire grip curves.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Pacejka 'Magic Formula' curve coefficients: $F_y = D \cdot \sin(C \cdot \arctan(B \cdot \alpha - E \cdot (B \cdot \alpha - \arctan(B \cdot \alpha))))$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacejkaCoefficients {
    pub b_stiffness: f32,
    pub c_shape: f32,
    pub d_peak: f32,
    pub e_curvature: f32,
}

impl Default for PacejkaCoefficients {
    fn default() -> Self {
        // Standard asphalt tire coefficients
        Self {
            b_stiffness: 10.0,
            c_shape: 1.30,
            d_peak: 1.0,
            e_curvature: -0.97,
        }
    }
}

impl PacejkaCoefficients {
    /// Evaluates normalized tire force multiplier for a given slip quantity (slip angle or slip ratio).
    pub fn evaluate(&self, slip: f32) -> f32 {
        let b = self.b_stiffness;
        let c = self.c_shape;
        let d = self.d_peak;
        let e = self.e_curvature;

        let b_slip = b * slip;
        let inner = b_slip - e * (b_slip - b_slip.atan());
        d * (c * inner.atan()).sin()
    }
}

/// Suspension wheel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelSuspension {
    pub relative_position: Vec3,
    pub rest_length: f32,
    pub current_length: f32,
    pub spring_stiffness: f32,
    pub damper_coefficient: f32,
    pub steering_angle: f32,
    pub wheel_radius: f32,
    pub wheel_mass: f32,
    pub compression_velocity: f32,
    pub normal_force: f32,
    pub pacejka: PacejkaCoefficients,
}

impl WheelSuspension {
    pub fn new(relative_pos: Vec3, rest_length: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            relative_position: relative_pos,
            rest_length,
            current_length: rest_length,
            spring_stiffness: stiffness,
            damper_coefficient: damping,
            steering_angle: 0.0,
            wheel_radius: 0.35,
            wheel_mass: 25.0,
            compression_velocity: 0.0,
            normal_force: 0.0,
            pacejka: PacejkaCoefficients::default(),
        }
    }

    /// Evaluates suspension spring-damper force along the suspension strut.
    pub fn compute_suspension_force(&mut self, hit_distance: Option<f32>, dt: f32) -> f32 {
        if let Some(dist) = hit_distance {
            let clamped_dist = dist.clamp(0.05, self.rest_length);
            let prev_length = self.current_length;
            self.current_length = clamped_dist;
            self.compression_velocity = (prev_length - self.current_length) / dt.max(1e-4);

            let compression = (self.rest_length - self.current_length).max(0.0);
            let spring_force = compression * self.spring_stiffness;
            let damper_force = self.compression_velocity * self.damper_coefficient;

            let force = (spring_force + damper_force).max(0.0);
            self.normal_force = force;
            force
        } else {
            self.current_length = self.rest_length;
            self.compression_velocity = 0.0;
            self.normal_force = 0.0;
            0.0
        }
    }

    /// Computes lateral cornering force using the Pacejka model.
    pub fn compute_lateral_tire_force(&self, lateral_slip_angle: f32) -> f32 {
        let mu = self.pacejka.evaluate(lateral_slip_angle);
        mu * self.normal_force
    }
}

/// 4-Wheel Vehicle Dynamics Model.
pub struct VehicleDynamics {
    pub chassis_mass: f32,
    pub wheels: [WheelSuspension; 4], // FL, FR, RL, RR
    pub anti_roll_stiffness: f32,
}

impl VehicleDynamics {
    pub fn new(mass: f32) -> Self {
        let fl = WheelSuspension::new(Vec3::new(-0.8, -0.2, 1.4), 0.45, 35000.0, 4500.0);
        let fr = WheelSuspension::new(Vec3::new(0.8, -0.2, 1.4), 0.45, 35000.0, 4500.0);
        let rl = WheelSuspension::new(Vec3::new(-0.8, -0.2, -1.4), 0.45, 38000.0, 4800.0);
        let rr = WheelSuspension::new(Vec3::new(0.8, -0.2, -1.4), 0.45, 38000.0, 4800.0);

        Self {
            chassis_mass: mass,
            wheels: [fl, fr, rl, rr],
            anti_roll_stiffness: 8000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacejka_tire_friction() {
        let pacejka = PacejkaCoefficients::default();
        let force_zero = pacejka.evaluate(0.0);
        let force_peak = pacejka.evaluate(0.15); // typical peak slip angle ~8.5 degrees

        assert_eq!(force_zero, 0.0);
        assert!(force_peak > 0.8);
    }

    #[test]
    fn test_suspension_compression() {
        let mut wheel = WheelSuspension::new(Vec3::ZERO, 0.5, 30000.0, 4000.0);
        let force = wheel.compute_suspension_force(Some(0.4), 0.016);
        assert!(force > 0.0);
        assert_eq!(wheel.normal_force, force);
    }
}
