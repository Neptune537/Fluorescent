import 'dart:typed_data';

/// Radar blip representation parsed from zero-copy shared memory.
class RadarBlipData {
  final int entityId;
  final double relativeX;
  final double relativeZ;
  final double altitudeOffset;
  final bool isHostile;
  final int blipType;

  const RadarBlipData({
    required this.entityId,
    required this.relativeX,
    required this.relativeZ,
    required this.altitudeOffset,
    required this.isHostile,
    required this.blipType,
  });

  factory RadarBlipData.fromByteData(ByteData data, int offset) {
    return RadarBlipData(
      entityId: data.getUint32(offset, Endian.little),
      relativeX: data.getFloat32(offset + 4, Endian.little),
      relativeZ: data.getFloat32(offset + 8, Endian.little),
      altitudeOffset: data.getFloat32(offset + 12, Endian.little),
      isHostile: data.getUint8(offset + 16) != 0,
      blipType: data.getUint8(offset + 17),
    );
  }
}

/// High-rate telemetry frame emitted by native simulation.
class TelemetrySnapshot {
  final int frameIndex;
  final int timestampUs;
  final double playerHealth;
  final double playerMaxHealth;
  final double playerShield;
  final double playerMaxShield;
  final double playerEnergy;
  final double playerMaxEnergy;
  final int heatLevel;
  final List<RadarBlipData> radarBlips;

  const TelemetrySnapshot({
    required this.frameIndex,
    required this.timestampUs,
    required this.playerHealth,
    required this.playerMaxHealth,
    required this.playerShield,
    required this.playerMaxShield,
    required this.playerEnergy,
    required this.playerMaxEnergy,
    required this.heatLevel,
    required this.radarBlips,
  });

  factory TelemetrySnapshot.fromByteData(ByteData data, [int offset = 0]) {
    final frameIndex = data.getUint64(offset, Endian.little);
    final timestampUs = data.getUint64(offset + 8, Endian.little);
    final health = data.getFloat32(offset + 16, Endian.little);
    final maxHealth = data.getFloat32(offset + 20, Endian.little);
    final shield = data.getFloat32(offset + 24, Endian.little);
    final maxShield = data.getFloat32(offset + 28, Endian.little);
    final energy = data.getFloat32(offset + 32, Endian.little);
    final maxEnergy = data.getFloat32(offset + 36, Endian.little);
    final heatLevel = data.getUint32(offset + 40, Endian.little);
    final blipCount = data.getUint32(offset + 44, Endian.little);

    final blips = <RadarBlipData>[];
    const blipSize = 20; // 4 + 4 + 4 + 4 + 1 + 1 + 2
    final blipStartOffset = offset + 48;

    for (var i = 0; i < blipCount && i < 32; i++) {
      blips.add(RadarBlipData.fromByteData(data, blipStartOffset + (i * blipSize)));
    }

    return TelemetrySnapshot(
      frameIndex: frameIndex,
      timestampUs: timestampUs,
      playerHealth: health,
      playerMaxHealth: maxHealth,
      playerShield: shield,
      playerMaxShield: maxShield,
      playerEnergy: energy,
      playerMaxEnergy: maxEnergy,
      heatLevel: heatLevel,
      radarBlips: blips,
    );
  }
}
