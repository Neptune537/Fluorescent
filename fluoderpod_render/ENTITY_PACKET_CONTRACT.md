# Entity Packet Contract (`EntityPacket64`)

Owner: Fluorescent (`fluoderpod_render`). Consumers: Fluoderpod SDK, Lite.
Implementation: `src/entity_packet.rs`.

Each entity is exactly **64 bytes, little-endian**, no padding. A batch is `N * 64` bytes.

| Offset | Size | Type | Field |
|-------:|-----:|------|-------|
| 0  | 8  | f64 | lat / x |
| 8  | 8  | f64 | lon / y |
| 16 | 8  | f64 | alt / z |
| 24 | 4  | u32 | type: 1 Task, 2 Player, 3 Guild Hub, 4 Trap |
| 28 | 4  | u32 | status |
| 32 | 4  | u32 | faction |
| 36 | 8  | u64 | bounty, micro-units |
| 44 | 20 | u8[20] | id |

Rust: `#[repr(C)] struct EntityPacket64` (bytemuck `Pod`, size 64, align 8; bounty is
stored as `[u8; 8]` LE since offset 36 is not 8-aligned). Decoding is
alignment-agnostic (`pod_read_unaligned`).

## Rejection
Byte length not divisible by 64 is an error (`EntityPacketError::InvalidLength`).
Empty input is valid (0 entities).

## FFI
- `fluoderpod_ingest_batch(renderer, ptr, len) -> i32`: 64-byte packets. Returns entity
  count (>= 0), `-1` null pointer, `-2` invalid length. Never panics.
- `fluoderpod_ingest_gpu_instances(...)`: legacy 48-byte GPU `PackedEntityInstance`
  buffers (internal GPU format, not the wire format). Same return codes.

## GPU decode
Packets are converted to the 48-byte `PackedEntityInstance`: position = (x, y, z) as f32,
radius 1.0, identity rotation, unit scale, `cluster_and_flags` =
`(faction & 0xFFFF) << 16 | (status & 0xFF) << 8 | (type & 0xFF)`.

## Host FPS control (thermal)
`thermal::set_target_fps(u32)`, `target_fps()`, `set_idle(bool)`, `is_idle()`;
FFI `fluoderpod_set_target_fps(u32)`, `fluoderpod_set_idle(bool)`. Tiers: 60 interactive,
24 static, 0 paused/idle.
