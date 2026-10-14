1. **Extend `telemetry_ring.rs` with Edge Compute Telemetry Data Structures**
    - Add `MAX_ACTIVE_LINKS` constant (e.g. 16 or 32).
    - Add `NodeRole` enum/representation (Master vs EdgeWorker).
    - Add `ProofOfHealth` struct to track `thermal_celsius`, `vram_allocated_pct`, `battery_pct`.
    - Add `ComputeThroughput` struct to track `tokens_per_sec`, `active_layers_assigned`, `kv_cache_bytes`.
    - Add `EdgeNodeTelemetry` struct combining node metadata (id, did, ECEF coords), role, `ProofOfHealth`, `ComputeThroughput`, and active links (array of target node ids).
    - Add `MAX_EDGE_NODES` constant.
    - Update `TelemetryFrame` to include an array of `EdgeNodeTelemetry` and a count `edge_node_count`.

2. **Verify Changes and Pre-commit steps**
    - Complete pre-commit steps to make sure proper testing, verifications, reviews and reflections are done.
    - Run tests inside `fluorite_core`.

3. **Submit the change.**
    - Submit the updated telemetry structures.
