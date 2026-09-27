//! Analytical Two-Bone IK and Iterative FABRIK Solvers.

use glam::{Quat, Vec3};

/// Joint representation in an inverse kinematics chain.
#[derive(Debug, Clone)]
pub struct IkJoint {
    pub position: Vec3,
    pub length: f32,
}

/// Output of analytical Two-Bone IK solver.
#[derive(Debug, Clone, Copy)]
pub struct TwoBoneIkResult {
    pub mid_joint_pos: Vec3,
    pub end_joint_pos: Vec3,
    pub is_reachable: bool,
}

/// Solves analytical Two-Bone IK (e.g. hip-knee-foot or shoulder-elbow-hand).
///
/// Uses Law of Cosines to calculate the exact mid-joint bend angle and pole vector projection.
pub fn solve_two_bone_ik(
    root_pos: Vec3,
    target_pos: Vec3,
    pole_target: Vec3,
    upper_length: f32,
    lower_length: f32,
) -> TwoBoneIkResult {
    let to_target = target_pos - root_pos;
    let target_dist = to_target.length();
    let max_reach = upper_length + lower_length;

    // Fully extended reach
    if target_dist >= max_reach - 1e-4 {
        let dir = to_target.normalize_or_zero();
        return TwoBoneIkResult {
            mid_joint_pos: root_pos + dir * upper_length,
            end_joint_pos: root_pos + dir * max_reach,
            is_reachable: false,
        };
    }

    // Law of Cosines: a^2 + b^2 - 2ab * cos(C) = c^2
    let a = upper_length;
    let b = lower_length;
    let c = target_dist.max((a - b).abs() + 1e-4);

    let cos_alpha = ((b * b + c * c - a * a) / (2.0 * b * c)).clamp(-1.0, 1.0);
    let cos_beta = ((a * a + c * c - b * b) / (2.0 * a * c)).clamp(-1.0, 1.0);
    let angle_beta = cos_beta.acos();

    // Construct reference plane using pole vector
    let target_dir = to_target / target_dist;
    let pole_dir = (pole_target - root_pos).normalize_or_zero();
    let bend_normal = target_dir.cross(pole_dir).normalize_or_zero();
    let mid_plane_dir = bend_normal.cross(target_dir).normalize_or_zero();

    // Rotate upper segment toward pole
    let mid_pos = root_pos + target_dir * (a * angle_beta.cos()) + mid_plane_dir * (a * angle_beta.sin());

    TwoBoneIkResult {
        mid_joint_pos: mid_pos,
        end_joint_pos: target_pos,
        is_reachable: true,
    }
}

/// Solves iterative FABRIK for arbitrary N-joint chains (e.g. spine, tail, tentacles).
pub fn solve_fabrik(
    joints: &mut [Vec3],
    lengths: &[f32],
    target: Vec3,
    max_iterations: usize,
    tolerance: f32,
) -> bool {
    let n = joints.len();
    if n < 2 || lengths.len() != n - 1 {
        return false;
    }

    let origin = joints[0];
    let total_length: f32 = lengths.iter().sum();
    let target_dist = (target - origin).length();

    // Unreachable: stretch chain directly toward target
    if target_dist > total_length {
        let dir = (target - origin).normalize_or_zero();
        for i in 0..(n - 1) {
            joints[i + 1] = joints[i] + dir * lengths[i];
        }
        return false;
    }

    // Reachable: iterate backwards and forwards
    for _ in 0..max_iterations {
        let end_dist = (joints[n - 1] - target).length();
        if end_dist <= tolerance {
            return true;
        }

        // Backward pass: from target to root
        joints[n - 1] = target;
        for i in (0..(n - 1)).rev() {
            let r = (joints[i] - joints[i + 1]).length().max(1e-4);
            let lambda = lengths[i] / r;
            joints[i] = (1.0 - lambda) * joints[i + 1] + lambda * joints[i];
        }

        // Forward pass: from root to target
        joints[0] = origin;
        for i in 0..(n - 1) {
            let r = (joints[i + 1] - joints[i]).length().max(1e-4);
            let lambda = lengths[i] / r;
            joints[i + 1] = (1.0 - lambda) * joints[i] + lambda * joints[i + 1];
        }
    }

    (joints[n - 1] - target).length() <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_bone_ik() {
        let root = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, 1.5);
        let pole = Vec3::new(0.0, 1.0, 0.5);

        let res = solve_two_bone_ik(root, target, pole, 1.0, 1.0);
        assert!(res.is_reachable);
        assert!((res.end_joint_pos - target).length() < 1e-4);
        assert!((res.mid_joint_pos - root).length() <= 1.01);
    }

    #[test]
    fn test_fabrik_chain() {
        let mut joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ];
        let lengths = vec![1.0, 1.0, 1.0];
        let target = Vec3::new(1.5, 1.5, 0.0);

        let solved = solve_fabrik(&mut joints, &lengths, target, 15, 0.01);
        assert!(solved);
        assert!((joints[3] - target).length() < 0.02);
    }
}
