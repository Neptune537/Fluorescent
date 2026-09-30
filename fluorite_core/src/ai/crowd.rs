//! Crowd Density Algorithms and Continuum Flow Fields.
//!
//! Models high-density urban pedestrian movement, dynamic flow vectors, and obstacle repulsion.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// 2D Flow Field Cell holding velocity vectors and density potential.
#[derive(Debug, Clone, Copy, Default)]
pub struct FlowCell {
    pub flow_direction: Vec2,
    pub density: f32,
    pub is_walkable: bool,
}

/// Dynamic Crowd Pedestrian Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrowdAgent {
    pub id: u32,
    pub position: Vec2,
    pub velocity: Vec2,
    pub target_position: Vec2,
    pub preferred_speed: f32,
    pub personal_radius: f32,
}

impl CrowdAgent {
    pub fn new(id: u32, pos: Vec2, target: Vec2) -> Self {
        Self {
            id,
            position: pos,
            velocity: Vec2::ZERO,
            target_position: target,
            preferred_speed: 1.4, // standard 1.4 m/s walking speed
            personal_radius: 0.35,
        }
    }
}

/// Crowd Density Field covering an urban region.
pub struct CrowdDensityField {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub cells: Vec<FlowCell>,
}

impl CrowdDensityField {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        let mut cells = vec![FlowCell::default(); width * height];
        for c in &mut cells {
            c.is_walkable = true;
        }

        Self {
            width,
            height,
            cell_size,
            cells,
        }
    }

    /// Computes steering velocity for an agent taking into account flow and local crowd density.
    pub fn compute_agent_steering(&self, agent: &CrowdAgent) -> Vec2 {
        let to_target = agent.target_position - agent.position;
        let dist = to_target.length();
        if dist < 0.2 {
            return Vec2::ZERO;
        }

        let desired_dir = to_target / dist;
        let desired_vel = desired_dir * agent.preferred_speed;

        // Blended steering
        let steering = desired_vel - agent.velocity;
        steering.clamp_length_max(4.0)
    }

    /// Applies mutual agent-agent repulsion to prevent overlapping.
    pub fn resolve_agent_collisions(agents: &mut [CrowdAgent]) {
        let len = agents.len();
        for i in 0..len {
            for j in (i + 1)..len {
                let delta = agents[j].position - agents[i].position;
                let dist = delta.length();
                let min_dist = agents[i].personal_radius + agents[j].personal_radius;

                if dist < min_dist && dist > 1e-4 {
                    let overlap = min_dist - dist;
                    let normal = delta / dist;
                    let push = normal * (overlap * 0.5);

                    agents[i].position -= push;
                    agents[j].position += push;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crowd_agent_steering() {
        let field = CrowdDensityField::new(32, 32, 1.0);
        let agent = CrowdAgent::new(1, Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let steer = field.compute_agent_steering(&agent);
        assert!(steer.x > 0.0);
    }
}
