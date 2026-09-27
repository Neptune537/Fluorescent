//! Motion Capture Animation & Inverse Kinematics (IK) Runtime.
//!
//! Provides analytical Two-Bone IK (arms, legs) and iterative FABRIK (Forwards and Backwards
//! Reaching Inverse Kinematics) for multi-joint chains blended with motion capture sequences.

pub mod ik;

pub use ik::{solve_fabrik, solve_two_bone_ik, IkJoint, TwoBoneIkResult};
