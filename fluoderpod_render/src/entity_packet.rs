//! `EntityPacket64`: the 64-byte little-endian wire format shared with the
//! Fluoderpod SDK and Lite. See `ENTITY_PACKET_CONTRACT.md`.
//!
//! This is distinct from the GPU-side 48-byte
//! [`crate::unified_pipeline::PackedEntityInstance`]; packets are decoded into it.

use crate::unified_pipeline::PackedEntityInstance;

#[cfg(target_endian = "big")]
compile_error!("EntityPacket64 is a little-endian wire format; big-endian hosts are unsupported");

pub const ENTITY_PACKET_SIZE: usize = 64;

pub const ENTITY_TYPE_TASK: u32 = 1;
pub const ENTITY_TYPE_PLAYER: u32 = 2;
pub const ENTITY_TYPE_GUILD_HUB: u32 = 3;
pub const ENTITY_TYPE_TRAP: u32 = 4;

/// Errors from decoding a packet batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPacketError {
    /// Byte length is not a multiple of 64.
    InvalidLength { len: usize },
}

impl std::fmt::Display for EntityPacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength { len } => write!(
                f,
                "Entity buffer byte length {} is not a multiple of {} bytes",
                len, ENTITY_PACKET_SIZE
            ),
        }
    }
}

impl std::error::Error for EntityPacketError {}

/// Wire entity. Offsets: 0 f64 x, 8 f64 y, 16 f64 z, 24 u32 type, 28 u32 status,
/// 32 u32 faction, 36 u64 bounty (micro-units, stored as LE bytes because offset 36
/// is not 8-aligned), 44 `[u8; 20]` id. Size 64, align 8, no padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EntityPacket64 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub entity_type: u32,
    pub status: u32,
    pub faction: u32,
    pub bounty_le: [u8; 8],
    pub id: [u8; 20],
}

const _: () = assert!(std::mem::size_of::<EntityPacket64>() == ENTITY_PACKET_SIZE);
const _: () = assert!(std::mem::align_of::<EntityPacket64>() == 8);

impl EntityPacket64 {
    pub fn bounty_micro(&self) -> u64 {
        u64::from_le_bytes(self.bounty_le)
    }

    pub fn set_bounty_micro(&mut self, v: u64) {
        self.bounty_le = v.to_le_bytes();
    }

    /// Decode into the GPU instance layout. Position is passed through as f32;
    /// low 16 bits of `cluster_and_flags` = `(status & 0xFF) << 8 | (type & 0xFF)`,
    /// high 16 bits = faction (used as material id).
    pub fn to_gpu_instance(&self) -> PackedEntityInstance {
        PackedEntityInstance {
            position: [self.x as f32, self.y as f32, self.z as f32],
            sphere_radius: 1.0,
            rotation_quat: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            cluster_and_flags: ((self.faction & 0xFFFF) << 16)
                | ((self.status & 0xFF) << 8)
                | (self.entity_type & 0xFF),
        }
    }
}

/// Decode a contiguous byte buffer (alignment-agnostic) into packets.
pub fn decode_packets(bytes: &[u8]) -> Result<Vec<EntityPacket64>, EntityPacketError> {
    if bytes.len() % ENTITY_PACKET_SIZE != 0 {
        return Err(EntityPacketError::InvalidLength { len: bytes.len() });
    }
    Ok(bytes
        .chunks_exact(ENTITY_PACKET_SIZE)
        .map(bytemuck::pod_read_unaligned::<EntityPacket64>)
        .collect())
}

/// Decode a packet batch straight into GPU instances.
pub fn decode_to_gpu_instances(bytes: &[u8]) -> Result<Vec<PackedEntityInstance>, EntityPacketError> {
    Ok(decode_packets(bytes)?.iter().map(|p| p.to_gpu_instance()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(i: u8) -> [u8; 64] {
        let mut b = [0u8; 64];
        b[0..8].copy_from_slice(&1.5f64.to_le_bytes());
        b[8..16].copy_from_slice(&(-2.25f64).to_le_bytes());
        b[16..24].copy_from_slice(&100.0f64.to_le_bytes());
        b[24..28].copy_from_slice(&4u32.to_le_bytes());
        b[28..32].copy_from_slice(&7u32.to_le_bytes());
        b[32..36].copy_from_slice(&0x0102u32.to_le_bytes());
        b[36..44].copy_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
        for k in 0..20 {
            b[44 + k] = i.wrapping_add(k as u8);
        }
        b
    }

    #[test]
    fn layout_offsets() {
        assert_eq!(std::mem::size_of::<EntityPacket64>(), 64);
        let b = build(0xA0);
        let p = decode_packets(&b).unwrap()[0];
        assert_eq!(p.x, 1.5);
        assert_eq!(p.y, -2.25);
        assert_eq!(p.z, 100.0);
        assert_eq!(p.entity_type, ENTITY_TYPE_TRAP);
        assert_eq!(p.status, 7);
        assert_eq!(p.faction, 0x0102);
        assert_eq!(p.bounty_micro(), 0x1122_3344_5566_7788);
        assert_eq!(p.id[0], 0xA0);
        assert_eq!(p.id[19], 0xA0u8.wrapping_add(19));
    }

    #[test]
    fn endianness_little() {
        let mut b = [0u8; 64];
        b[24] = 0x01; // LE: low byte first
        let p = decode_packets(&b).unwrap()[0];
        assert_eq!(p.entity_type, 1);
        b[24] = 0;
        b[27] = 0x01;
        assert_eq!(decode_packets(&b).unwrap()[0].entity_type, 0x0100_0000);
    }

    #[test]
    fn rejects_bad_lengths() {
        for n in [1usize, 48, 63, 65, 96, 127] {
            assert_eq!(
                decode_packets(&vec![0u8; n]),
                Err(EntityPacketError::InvalidLength { len: n })
            );
        }
        assert!(decode_packets(&[]).unwrap().is_empty());
        assert_eq!(decode_packets(&vec![0u8; 192]).unwrap().len(), 3);
    }

    #[test]
    fn round_trip_and_unaligned() {
        let mut buf = vec![0u8; 1];
        buf.extend_from_slice(&build(1));
        buf.extend_from_slice(&build(2));
        let pk = decode_packets(&buf[1..]).unwrap(); // deliberately unaligned slice
        assert_eq!(pk.len(), 2);
        let mut out = Vec::new();
        for p in &pk {
            out.extend_from_slice(bytemuck::bytes_of(p));
        }
        assert_eq!(&out[..], &buf[1..]);
    }

    #[test]
    fn gpu_decode() {
        let g = decode_to_gpu_instances(&build(0)).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].position, [1.5, -2.25, 100.0]);
        assert_eq!(g[0].cluster_and_flags, (0x0102 << 16) | (7 << 8) | 4);
    }
}
