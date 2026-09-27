//! Ultra-Low Latency Input Pipeline & Command Marshaling.
//!
//! Marshals keyboard, mouse, gamepad, and multi-touch touch events from Flutter into
//! Fluorite's deterministic simulation tick queue with sub-millisecond overhead.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyCode {
    KeyW, KeyA, KeyS, KeyD,
    KeySpace, KeyShift, KeyCtrl,
    KeyE, KeyR, KeyF, KeyQ,
    KeyEsc, KeyTab,
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GamepadState {
    pub left_stick: [f32; 2],  // [-1.0, 1.0]
    pub right_stick: [f32; 2], // [-1.0, 1.0]
    pub left_trigger: f32,     // [0.0, 1.0]
    pub right_trigger: f32,    // [0.0, 1.0]
    pub buttons_bitmask: u32,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            left_stick: [0.0, 0.0],
            right_stick: [0.0, 0.0],
            left_trigger: 0.0,
            right_trigger: 0.0,
            buttons_bitmask: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEventType {
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    MouseMove { delta_x: f32, delta_y: f32 },
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseWheel { scroll_y: f32 },
    GamepadUpdate(GamepadState),
    TouchDown { pointer_id: u32, pos_x: f32, pos_y: f32 },
    TouchMove { pointer_id: u32, pos_x: f32, pos_y: f32 },
    TouchUp { pointer_id: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputCommand {
    pub sequence_number: u64,
    pub timestamp_us: u64,
    pub event: InputEventType,
}

/// Simulation tick input queue.
pub struct InputCommandQueue {
    queue: VecDeque<InputCommand>,
    sequence_counter: u64,
}

impl InputCommandQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(128),
            sequence_counter: 0,
        }
    }

    /// Enqueues an input event from Flutter/Dart.
    pub fn push_event(&mut self, event: InputEventType, timestamp_us: u64) {
        self.sequence_counter += 1;
        self.queue.push_back(InputCommand {
            sequence_number: self.sequence_counter,
            timestamp_us,
            event,
        });
    }

    /// Drains all accumulated input events for the upcoming simulation tick.
    pub fn drain_tick_events(&mut self) -> Vec<InputCommand> {
        self.queue.drain(..).collect()
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_queue() {
        let mut queue = InputCommandQueue::new();
        queue.push_event(InputEventType::KeyDown(KeyCode::KeyW), 1000);
        queue.push_event(InputEventType::MouseMove { delta_x: 2.5, delta_y: -1.0 }, 1005);

        assert_eq!(queue.pending_count(), 2);
        let drained = queue.drain_tick_events();
        assert_eq!(drained.len(), 2);
        assert_eq!(queue.pending_count(), 0);
    }
}
