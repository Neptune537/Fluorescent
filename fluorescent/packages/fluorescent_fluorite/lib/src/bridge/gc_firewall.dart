import 'dart:typed_data';

/// GC Firewall Sanitized State for zero-GC Flutter frame presentation.
///
/// Ensures strict isolation: zero 3D matrices or ECS entity handles ever cross into Dart,
/// eliminating GC pauses and frame hitching.
class SanitizedUiSnapshot {
  final double compassHeadingDegrees;
  final double speedKmh;
  final double altitudeMeters;
  final int currentAmmo;
  final int reserveAmmo;
  final double weaponHeatPercentage;
  final bool targetLocked;
  final double targetLeadX;
  final double targetLeadY;
  final double activeQuestDistance;

  const SanitizedUiSnapshot({
    required this.compassHeadingDegrees,
    required this.speedKmh,
    required this.altitudeMeters,
    required this.currentAmmo,
    required this.reserveAmmo,
    required this.weaponHeatPercentage,
    required this.targetLocked,
    required this.targetLeadX,
    required this.targetLeadY,
    required this.activeQuestDistance,
  });

  factory SanitizedUiSnapshot.fromByteData(ByteData data, [int offset = 0]) {
    return SanitizedUiSnapshot(
      compassHeadingDegrees: data.getFloat32(offset, Endian.little),
      speedKmh: data.getFloat32(offset + 4, Endian.little),
      altitudeMeters: data.getFloat32(offset + 8, Endian.little),
      currentAmmo: data.getUint32(offset + 12, Endian.little),
      reserveAmmo: data.getUint32(offset + 16, Endian.little),
      weaponHeatPercentage: data.getFloat32(offset + 20, Endian.little),
      targetLocked: data.getUint32(offset + 24, Endian.little) != 0,
      targetLeadX: data.getFloat32(offset + 28, Endian.little),
      targetLeadY: data.getFloat32(offset + 32, Endian.little),
      activeQuestDistance: data.getFloat32(offset + 36, Endian.little),
    );
  }

  factory SanitizedUiSnapshot.mock() {
    return const SanitizedUiSnapshot(
      compassHeadingDegrees: 45.0,
      speedKmh: 82.5,
      altitudeMeters: 140.0,
      currentAmmo: 28,
      reserveAmmo: 180,
      weaponHeatPercentage: 0.15,
      targetLocked: true,
      targetLeadX: 0.08,
      targetLeadY: -0.04,
      activeQuestDistance: 320.0,
    );
  }
}
