# Reticulum Bridge Architecture

The **Reticulum Bridge** connects the `fluoderpod_render` GPU-driven core directly to the Reticulum Network Stack (RNS) mesh network. It bypasses Dart FFI and traditional TCP/WebSocket dependencies to provide native, offline-first mesh networking for the **Nexus Protocol (Fluoridians Network)**.

## Core Capabilities

The bridge operates inside the native Rust core and provides two primary capabilities:

1.  **Zero-Overhead Broadcasting (Telemetry):**
    When the local ECS state updates (such as high-throughput batch ingestion of instances), the bridge immediately broadcasts `NexusMessage::Telemetry` over the Reticulum mesh.
2.  **Non-Blocking Ingestion (TensorDiff):**
    Remote peers on the mesh network can send state changes (`TensorDiff`) or UI adjustments. These are captured by the Reticulum listener and pushed to a non-blocking queue.

## Implementation Details

### Location
The module is located at `fluoderpod_render/src/reticulum_bridge.rs`. It is conditionally compiled out on `wasm32` targets, as it relies on native socket bridging.

### Initialization
The bridge relies on `tokio::sync::mpsc` for inter-task communication. When `ReticulumBridge::initialize()` is called:
1. It establishes a connection to the Reticulum identity.
2. It spawns a background `tokio` task to listen for incoming packets asynchronously.
3. Packets are parsed into `NexusMessage` enums and forwarded via the MPSC channel.

### Scanning & Receiving Messages
To keep the renderer's tick loop fully decoupled and free from IO blocking, the bridge provides the `try_recv_message()` method.

```rust
if let Some(bridge) = &renderer.reticulum_bridge {
    while let Some(msg) = bridge.try_recv_message() {
        match msg {
            NexusMessage::TensorDiff { diff_buffer, node_id } => {
                // Apply the structural diff directly to GPU buffers
            }
            NexusMessage::Telemetry { payload } => {
                // Log or apply external UI adjustments
            }
            NexusMessage::Ping => {}
        }
    }
}
```

By draining this queue at the start of each frame inside `execute_frame` or before `ingest_fluoderpod_batch`, the engine maintains high GPU throughput while participating seamlessly in the mesh network.

## Dependencies
- `reticulum = "0.1"`: The native Rust implementation of the Reticulum stack.
- `tokio`: Async runtime used to handle the mesh listeners.
- `serde` / `serde_json`: Used for parsing the tagged-enum Nexus Protocol payload.
