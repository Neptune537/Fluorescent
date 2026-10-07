use serde::{Deserialize, Serialize};

/// Identifies a specific geofence instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeofenceId(pub String);

/// Defines the shape and boundaries of a geofence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GeofenceShape {
    /// Axis-Aligned Bounding Box
    Aabb {
        min: [f32; 3],
        max: [f32; 3],
    },
    /// Spherical boundary
    Sphere {
        center: [f32; 3],
        radius: f32,
    },
    /// 2D Polygon extruded along the Y axis
    ExtrudedPolygon {
        /// X and Z coordinates of the polygon vertices
        vertices: Vec<[f32; 2]>,
        y_min: f32,
        y_max: f32,
    },
}

impl GeofenceShape {
    /// Checks if a 3D point is inside the boundary.
    pub fn contains(&self, point: [f32; 3]) -> bool {
        match self {
            Self::Aabb { min, max } => {
                point[0] >= min[0] && point[0] <= max[0] &&
                point[1] >= min[1] && point[1] <= max[1] &&
                point[2] >= min[2] && point[2] <= max[2]
            }
            Self::Sphere { center, radius } => {
                let dx = point[0] - center[0];
                let dy = point[1] - center[1];
                let dz = point[2] - center[2];
                (dx * dx + dy * dy + dz * dz) <= (radius * radius)
            }
            Self::ExtrudedPolygon { vertices, y_min, y_max } => {
                if point[1] < *y_min || point[1] > *y_max {
                    return false;
                }
                
                // Ray-casting algorithm for point-in-polygon (2D: X and Z)
                let mut inside = false;
                let mut j = vertices.len() - 1;
                for i in 0..vertices.len() {
                    let vi = vertices[i];
                    let vj = vertices[j];
                    
                    if (vi[1] > point[2]) != (vj[1] > point[2]) {
                        let intersect_x = (vj[0] - vi[0]) * (point[2] - vi[1]) / (vj[1] - vi[1]) + vi[0];
                        if point[0] < intersect_x {
                            inside = !inside;
                        }
                    }
                    j = i;
                }
                inside
            }
        }
    }
}

/// A serialized geofence layer for the editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geofence {
    pub id: GeofenceId,
    pub shape: GeofenceShape,
    /// Arbitrary tags for gameplay logic (e.g., "safe_zone", "pvp_area", "water")
    pub tags: Vec<String>,
}

/// A collection of geofences, typically loaded from an editor serialization format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeofenceRegistry {
    pub fences: Vec<Geofence>,
}

impl GeofenceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from a JSON string provided by the editor/Flutter.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON to send back to the editor/Flutter.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Add or update a geofence.
    pub fn upsert(&mut self, geofence: Geofence) {
        if let Some(existing) = self.fences.iter_mut().find(|f| f.id == geofence.id) {
            *existing = geofence;
        } else {
            self.fences.push(geofence);
        }
    }

    /// Remove a geofence by ID.
    pub fn remove(&mut self, id: &GeofenceId) {
        self.fences.retain(|f| &f.id != id);
    }

    /// Returns a list of geofence IDs that contain the given point.
    pub fn query_point(&self, point: [f32; 3]) -> Vec<GeofenceId> {
        self.fences
            .iter()
            .filter(|f| f.shape.contains(point))
            .map(|f| f.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geofence_serialization() {
        let registry = GeofenceRegistry {
            fences: vec![
                Geofence {
                    id: GeofenceId("safe_zone_01".to_string()),
                    shape: GeofenceShape::Aabb {
                        min: [-10.0, 0.0, -10.0],
                        max: [10.0, 20.0, 10.0],
                    },
                    tags: vec!["no_pvp".to_string()],
                },
                Geofence {
                    id: GeofenceId("boss_arena".to_string()),
                    shape: GeofenceShape::Sphere {
                        center: [100.0, 0.0, 100.0],
                        radius: 50.0,
                    },
                    tags: vec!["combat_lock".to_string()],
                },
            ],
        };

        let json = registry.to_json().unwrap();
        let loaded = GeofenceRegistry::from_json(&json).unwrap();
        
        assert_eq!(registry.fences.len(), loaded.fences.len());
        assert_eq!(registry.fences[0].id, loaded.fences[0].id);
    }

    #[test]
    fn test_point_containment() {
        let sphere = GeofenceShape::Sphere { center: [0.0, 0.0, 0.0], radius: 5.0 };
        assert!(sphere.contains([3.0, 0.0, 4.0]));
        assert!(!sphere.contains([5.0, 5.0, 5.0]));

        let polygon = GeofenceShape::ExtrudedPolygon {
            vertices: vec![
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 10.0],
                [0.0, 10.0],
            ],
            y_min: 0.0,
            y_max: 10.0,
        };
        assert!(polygon.contains([5.0, 5.0, 5.0]));
        assert!(!polygon.contains([15.0, 5.0, 5.0]));
        assert!(!polygon.contains([5.0, 15.0, 5.0]));
    }
}
