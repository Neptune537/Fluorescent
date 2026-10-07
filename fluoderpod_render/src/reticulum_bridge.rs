use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

// Note: These structs are simulated abstractions for the reticulum-rs crate.
// Depending on the exact version and crate used (e.g., `reticulum` or `Rusticulum`),
// the initialization and types will slightly vary.

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum NexusMessage {
    Telemetry { payload: String },
    MapUpdate { json_payload: String },
    TensorDiff { diff_buffer: String, node_id: String },
    LocationUpdate {
        entity_id: String,
        lat: f64,
        lng: f64,
        timestamp: i64,
        flight: Option<String>,
    },
    Ping,
}

pub struct ReticulumBridge {
    // This channel is used to send messages received from Reticulum back to the main Engine/ECS.
    pub rx_channel: Arc<Mutex<mpsc::Receiver<NexusMessage>>>,
}

impl ReticulumBridge {
    pub async fn initialize() -> Result<Self, Box<dyn std::error::Error>> {
        let (_tx, rx) = mpsc::channel::<NexusMessage>(100);

        // In a real implementation with `reticulum`, you would:
        // 1. Initialize the Reticulum stack: `let mut rns = Reticulum::new();`
        // 2. Create a destination/identity: `let dest = Destination::new(...)`
        // 3. Register a callback or loop over incoming packets.
        // 4. Deserialize incoming packets into `NexusMessage` and send via `tx`.

        // Example mock loop simulating an incoming message:
        // let tx_clone = _tx.clone();
        tokio::spawn(async move {
            // Simulated connection listener
            log::info!("Reticulum bridge listening for Nexus Protocol messages...");
            // let packet = dest.receive().await;
            // if let Ok(msg) = serde_json::from_slice(&packet.data) { ... }
        });

        Ok(Self {
            rx_channel: Arc::new(Mutex::new(rx)),
        })
    }

    pub fn send_nexus_message(&self, message: NexusMessage) {
        // Serialize the message and broadcast it to the Reticulum mesh.
        match serde_json::to_vec(&message) {
            Ok(_data) => {
                // let packet = Packet::new(destination, data);
                // packet.send();
                log::debug!("Sent NexusMessage over Reticulum: {:?}", message);
            }
            Err(e) => {
                log::error!("Failed to serialize NexusMessage for Reticulum: {}", e);
            }
        }
    }

    /// Scans the receive channel for any newly arrived Nexus Protocol messages from the mesh network.
    /// This is non-blocking and meant to be called during the renderer's tick/frame loop.
    pub fn try_recv_message(&self) -> Option<NexusMessage> {
        if let Ok(mut rx) = self.rx_channel.try_lock() {
            if let Ok(message) = rx.try_recv() {
                return Some(message);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reticulum_bridge_initialize() {
        let bridge = ReticulumBridge::initialize().await;
        assert!(bridge.is_ok());
    }

    #[tokio::test]
    async fn test_try_recv_message_empty() {
        let bridge = ReticulumBridge::initialize().await.unwrap();
        assert_eq!(bridge.try_recv_message(), None);
    }
}
