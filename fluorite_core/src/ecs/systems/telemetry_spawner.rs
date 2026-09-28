use crate::ecs::components::geo::{Appearance, Controller, Visibility, WorldEntity, Location};
use crate::fluoropod::{InputCommandQueue, InputEventType};
use crate::spatial::geo::coords::lat_lon_alt_to_ecef;
// use crate::spatial::geo::coords::ecef_to_enu; // Depending on how you establish the local ENU origin
use hecs::World;

pub struct TelemetrySpawnerSystem;

impl TelemetrySpawnerSystem {
    /// Polls the input queue for telemetry network events (e.g. `SpawnGeoNode`)
    /// and spawns `WorldEntity` components into the ECS.
    pub fn process_telemetry(
        ecs: &mut World,
        input_queue: &mut InputCommandQueue,
        // local_origin_ecef: glam::DVec3, // Origin for ENU conversion
    ) {
        let events = input_queue.drain_tick_events();
        
        for cmd in events {
            if let InputEventType::SpawnGeoNode { id, lat, lon, alt } = cmd.event {
                // 1. Create the base entity struct
                let mut entity = WorldEntity::new(
                    id,
                    lat,
                    lon,
                    alt,
                    Visibility::Exact,
                    Controller::Ai,
                    Appearance::Invisible, // Or some default model
                );
                
                // 2. Perform geographic math. (WGS84 -> ECEF -> ENU)
                // For simplicity here, we assume lat_lon_alt_to_ecef returns a translation
                let ecef = lat_lon_alt_to_ecef(lat, lon, alt);
                
                // In a full implementation, you'd subtract the local engine origin (in ECEF)
                // and multiply by the ENU rotation matrix.
                let local_x = ecef.x as f32; // Placeholder
                let local_y = ecef.y as f32;
                let local_z = ecef.z as f32;
                
                let transform = glam::Mat4::from_translation(glam::Vec3::new(local_x, local_y, local_z));
                entity.transform = transform.to_cols_array();
                
                // 3. Spawn into the ECS with the `Location` marker component
                ecs.spawn((entity, Location));
            } else {
                // If it's a regular input event, push it back or route to the combat/movement systems.
                // Normally you'd process this earlier or have a dedicated input routing system.
                // For now, we will ignore non-SpawnGeoNode events here.
            }
        }
    }
}
