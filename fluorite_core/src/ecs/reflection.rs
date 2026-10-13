//! Runtime reflection system powered by `bevy_reflect`.
//!
//! Provides dynamic component inspection, property discovery, and serialization
//! for the `fluorite_editor` Flutter desktop inspector without manual FFI boilerplate.

use bevy_reflect::serde::{ReflectDeserializer, ReflectSerializer};
use bevy_reflect::{GetTypeRegistration, Reflect, TypeRegistryArc};
use serde::de::DeserializeSeed;

pub use bevy_reflect;

/// Global type registry for Fluorescent engine components and assets.
pub struct EngineTypeRegistry {
    pub registry: TypeRegistryArc,
}

impl Default for EngineTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineTypeRegistry {
    /// Creates a new `EngineTypeRegistry` and registers standard Fluorescent components.
    pub fn new() -> Self {
        let registry = TypeRegistryArc::default();
        let mut reg = Self { registry };
        reg.register_standard_types();
        reg
    }

    /// Registers standard types into the reflection system.
    pub fn register_standard_types(&mut self) {
        // Register primitives and core structs
        let mut write = self.registry.write();
        write.register::<crate::ecs::world::TransformComponent>();
        write.register::<crate::ecs::components::geo::GeographicPosition>();
        write.register::<crate::ecs::components::geo::Visibility>();
        write.register::<crate::ecs::components::geo::Controller>();
        write.register::<crate::ecs::components::geo::Appearance>();
        write.register::<crate::ecs::components::geo::WorldEntity>();
    }

    /// Register an arbitrary type implementing `Reflect` and `GetTypeRegistration`.
    pub fn register<T: Reflect + GetTypeRegistration>(&mut self) {
        self.registry.write().register::<T>();
    }

    /// Serialize any reflected value to a JSON string using Bevy's `ReflectSerializer`.
    pub fn serialize_to_json(&self, value: &dyn Reflect) -> Result<String, String> {
        let read = self.registry.read();
        let serializer = ReflectSerializer::new(value, &read);
        serde_json::to_string_pretty(&serializer).map_err(|e| e.to_string())
    }

    /// Deserialize a JSON string back into a boxed dynamic reflected value.
    pub fn deserialize_from_json(
        &self,
        json_str: &str,
    ) -> Result<Box<dyn bevy_reflect::PartialReflect>, String> {
        let read = self.registry.read();
        let mut deserializer = serde_json::Deserializer::from_str(json_str);
        let reflect_deserializer = ReflectDeserializer::new(&read);
        reflect_deserializer
            .deserialize(&mut deserializer)
            .map_err(|e| e.to_string())
    }

    /// Dynamically inspect and extract a field value as a debug string from any reflected struct.
    pub fn inspect_field(value: &dyn Reflect, field_name: &str) -> Option<String> {
        if let Ok(reflect_struct) = value.reflect_ref().as_struct() {
            reflect_struct.field(field_name).map(|f| format!("{:?}", f))
        } else {
            None
        }
    }

    /// Mutates a named field in a reflected struct using another dynamic reflected value.
    pub fn mutate_field(
        value: &mut dyn Reflect,
        field_name: &str,
        new_field_value: &dyn bevy_reflect::PartialReflect,
    ) -> Result<(), String> {
        if let Ok(reflect_struct) = value.reflect_mut().as_struct() {
            if let Some(target_field) = reflect_struct.field_mut(field_name) {
                target_field.apply(new_field_value);
                Ok(())
            } else {
                Err(format!("Field '{}' not found on struct", field_name))
            }
        } else {
            Err("Reflected value is not a struct".to_string())
        }
    }
}
