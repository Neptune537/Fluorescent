use serde::{Deserialize, Serialize};

/// Represents a physical location on the Earth using WGS84.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bevy_reflect::Reflect, bevy_ecs::component::Component)]
pub struct GeographicPosition {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
}

impl GeographicPosition {
    pub fn new(lat: f64, lon: f64, alt: f64) -> Self {
        Self { lat, lon, alt }
    }

    /// Converts this geographic coordinate to a Cesium Cartesian3 ECEF vector.
    pub fn to_cesium_cartesian3(&self) -> fluoridian_core::fluoderpod::cesium::CesiumCartesian3 {
        fluoridian_core::fluoderpod::cesium::CesiumCartesian3::from_degrees(self.lon, self.lat, self.alt)
    }

    /// Converts this geographic coordinate to a Cesium Cartographic representation.
    pub fn to_cesium_cartographic(&self) -> fluoridian_core::fluoderpod::cesium::CesiumCartographic {
        fluoridian_core::fluoderpod::cesium::CesiumCartographic {
            longitude: self.lon,
            latitude: self.lat,
            height: self.alt,
        }
    }

    /// Converts this coordinate into a lightweight CZML packet for streaming to CesiumJS.
    pub fn to_czml_packet(
        &self,
        node_id: &str,
        is_relay: bool,
    ) -> fluoridian_core::fluoderpod::cesium::CzmlPacket {
        fluoridian_core::fluoderpod::cesium::CzmlPacket::from_peer_node(
            node_id,
            self.lat,
            self.lon,
            self.alt,
            is_relay,
            None,
        )
    }
}

/// The visibility level of an entity, useful for privacy filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, bevy_reflect::Reflect)]
pub enum Visibility {
    Exact,
    Approximate,
    Neighborhood,
    City,
    Hidden,
}

/// Defines what controls the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, bevy_reflect::Reflect)]
pub enum Controller {
    Human,
    Ai,
    None,
}

/// Defines the appearance of the entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, bevy_reflect::Reflect)]
pub enum Appearance {
    Avatar(String),
    Model(String),
    Invisible,
}

/// Marker component for a Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player;

/// Marker component for an NPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Npc;

/// Marker component for a static Location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location;

/// A monolithic entity abstraction holding all geospatial and semantic game state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bevy_reflect::Reflect, bevy_ecs::component::Component)]
pub struct WorldEntity {
    pub id: u64,
    pub geographic_position: GeographicPosition,
    pub transform: [f32; 16],
    pub visibility: Visibility,
    pub controller: Controller,
    pub appearance: Appearance,
    #[reflect(ignore)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl WorldEntity {
    pub fn new(
        id: u64,
        lat: f64,
        lon: f64,
        alt: f64,
        visibility: Visibility,
        controller: Controller,
        appearance: Appearance,
    ) -> Self {
        Self {
            id,
            geographic_position: GeographicPosition::new(lat, lon, alt),
            transform: glam::Mat4::IDENTITY.to_cols_array(),
            visibility,
            controller,
            appearance,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Converts the WorldEntity into a streaming CZML packet for CesiumJS rendering.
    pub fn to_czml_packet(&self) -> fluoridian_core::fluoderpod::cesium::CzmlPacket {
        let is_relay = match self.controller {
            Controller::Human | Controller::Ai => false,
            Controller::None => true,
        };
        self.geographic_position.to_czml_packet(&self.id.to_string(), is_relay)
    }
}
