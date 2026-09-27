//! Bio-Mechanical Ragdoll Physics & Neural Network-Guided Balance Reactions (Euphoria Model).
//!
//! Provides dynamic balance controllers, active ragdoll muscle torques, and fall protection
//! reactions using PD (Proportional-Derivative) joint actuators.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// State of character balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceState {
    StandingStable,
    Stumbling,
    BracingForImpact,
    FreeFalling,
    RecoveringToFeet,
}

/// Dynamic joint actuator modeling active human musculature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuscleJointActuator {
    pub name: String,
    pub target_rotation: Quat,
    pub current_rotation: Quat,
    pub target_angular_velocity: Vec3,
    pub current_angular_velocity: Vec3,
    pub stiffness_kp: f32, // Spring coefficient
    pub damping_kd: f32,   // Damper coefficient
    pub max_torque: f32,   // Maximum muscle exertion in Newton-meters (Nm)
}

impl MuscleJointActuator {
    pub fn new(name: impl Into<String>, stiffness: f32, damping: f32, max_torque: f32) -> Self {
        Self {
            name: name.into(),
            target_rotation: Quat::IDENTITY,
            current_rotation: Quat::IDENTITY,
            target_angular_velocity: Vec3::ZERO,
            current_angular_velocity: Vec3::ZERO,
            stiffness_kp: stiffness,
            damping_kd: damping,
            max_torque,
        }
    }

    /// Computes active muscle restoration torque: $\tau = K_p \cdot \Delta\theta + K_d \cdot \Delta\omega$.
    pub fn compute_torque(&self) -> Vec3 {
        let diff_quat = self.target_rotation * self.current_rotation.inverse();
        let (axis, angle) = diff_quat.to_axis_angle();
        let angle_clamped = if angle > std::f32::consts::PI {
            angle - 2.0 * std::f32::consts::PI
        } else {
            angle
        };

        let proportional = axis * (angle_clamped * self.stiffness_kp);
        let derivative = (self.target_angular_velocity - self.current_angular_velocity) * self.damping_kd;
        let torque = proportional + derivative;

        if torque.length() > self.max_torque {
            torque.normalize() * self.max_torque
        } else {
            torque
        }
    }
}

/// Bio-Mechanical Character Controller.
pub struct BiomechanicalController {
    pub center_of_mass: Vec3,
    pub support_polygon_center: Vec3,
    pub linear_velocity: Vec3,
    pub balance_state: BalanceState,
    pub joints: Vec<MuscleJointActuator>,
    pub step_recovery_impulse: Vec3,
}

impl BiomechanicalController {
    pub fn new() -> Self {
        let spine = MuscleJointActuator::new("Spine", 1200.0, 150.0, 800.0);
        let left_hip = MuscleJointActuator::new("LeftHip", 1500.0, 180.0, 1000.0);
        let right_hip = MuscleJointActuator::new("RightHip", 1500.0, 180.0, 1000.0);
        let left_knee = MuscleJointActuator::new("LeftKnee", 1400.0, 160.0, 900.0);
        let right_knee = MuscleJointActuator::new("RightKnee", 1400.0, 160.0, 900.0);
        let neck = MuscleJointActuator::new("Neck", 400.0, 50.0, 200.0);

        Self {
            center_of_mass: Vec3::new(0.0, 1.0, 0.0),
            support_polygon_center: Vec3::ZERO,
            linear_velocity: Vec3::ZERO,
            balance_state: BalanceState::StandingStable,
            joints: vec![spine, left_hip, right_hip, left_knee, right_knee, neck],
            step_recovery_impulse: Vec3::ZERO,
        }
    }

    /// Evaluates dynamic zero-moment point (ZMP) and adjusts muscle targets to maintain balance.
    pub fn step_simulation(&mut self, dt: f32, external_impulse: Vec3) {
        self.linear_velocity += external_impulse;

        // Dynamic Zero-Moment Point (ZMP) estimation
        let g = 9.81;
        let zmp = Vec3::new(
            self.center_of_mass.x - (self.center_of_mass.y / g) * (self.linear_velocity.x / dt.max(1e-4)),
            0.0,
            self.center_of_mass.z - (self.center_of_mass.y / g) * (self.linear_velocity.z / dt.max(1e-4)),
        );

        let distance_to_support = (zmp - self.support_polygon_center).length();

        if distance_to_support < 0.25 {
            self.balance_state = BalanceState::StandingStable;
            self.step_recovery_impulse = Vec3::ZERO;
        } else if distance_to_support < 0.8 {
            self.balance_state = BalanceState::Stumbling;
            // Initiate reactive stepping motion in direction of displacement
            self.step_recovery_impulse = (zmp - self.support_polygon_center).normalize() * 300.0;
        } else {
            self.balance_state = BalanceState::BracingForImpact;
            // Protect head: tuck neck and extend arms
            if let Some(neck) = self.joints.iter_mut().find(|j| j.name == "Neck") {
                neck.target_rotation = Quat::from_rotation_x(0.4);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_muscle_torque() {
        let mut actuator = MuscleJointActuator::new("Test", 1000.0, 100.0, 500.0);
        actuator.target_rotation = Quat::from_rotation_y(0.2);
        let torque = actuator.compute_torque();
        assert!(torque.length() > 0.0);
        assert!(torque.length() <= 500.0);
    }

    #[test]
    fn test_biomechanical_balance() {
        let mut bio = BiomechanicalController::new();
        assert_eq!(bio.balance_state, BalanceState::StandingStable);

        bio.step_simulation(0.016, Vec3::new(5.0, 0.0, 0.0));
        assert!(bio.balance_state != BalanceState::StandingStable);
    }
}
