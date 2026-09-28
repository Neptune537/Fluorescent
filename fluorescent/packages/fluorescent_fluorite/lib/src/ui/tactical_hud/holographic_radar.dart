import 'dart:math' as math;
import 'package:flutter/widgets.dart';
import '../../bridge/telemetry_stream.dart';

/// Holographic tactical radar and minimap overlay.
class HolographicRadar extends StatelessWidget {
  final double headingDegrees;
  final List<RadarBlipData> blips;
  final double radarRadius;
  final double maxRangeMeters;

  const HolographicRadar({
    super.key,
    required this.headingDegrees,
    required this.blips,
    this.radarRadius = 90.0,
    this.maxRangeMeters = 200.0,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: radarRadius * 2,
      height: radarRadius * 2,
      child: CustomPaint(
        painter: _RadarPainter(
          headingDegrees: headingDegrees,
          blips: blips,
          radarRadius: radarRadius,
          maxRangeMeters: maxRangeMeters,
        ),
      ),
    );
  }
}

class _RadarPainter extends CustomPainter {
  final double headingDegrees;
  final List<RadarBlipData> blips;
  final double radarRadius;
  final double maxRangeMeters;

  _RadarPainter({
    required this.headingDegrees,
    required this.blips,
    required this.radarRadius,
    required this.maxRangeMeters,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width * 0.5, size.height * 0.5);

    // 1. Radar Background Disk
    final bgPaint = Paint()
      ..color = const Color(0xCC05101E)
      ..style = PaintingStyle.fill;
    canvas.drawCircle(center, radarRadius, bgPaint);

    // 2. Concentric Range Rings
    final ringPaint = Paint()
      ..color = const Color(0x3300E5FF)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;

    canvas.drawCircle(center, radarRadius * 0.33, ringPaint);
    canvas.drawCircle(center, radarRadius * 0.66, ringPaint);
    canvas.drawCircle(center, radarRadius, ringPaint);

    // 3. Compass Crosshairs and Cardinal Points
    final headingRad = (headingDegrees * math.pi) / 180.0;
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(-headingRad);

    final linePaint = Paint()
      ..color = const Color(0x2200E5FF)
      ..strokeWidth = 1.0;

    canvas.drawLine(Offset(-radarRadius, 0), Offset(radarRadius, 0), linePaint);
    canvas.drawLine(Offset(0, -radarRadius), Offset(0, radarRadius), linePaint);

    // North Indicator Arrow
    final northPaint = Paint()
      ..color = const Color(0xFFFF5252)
      ..style = PaintingStyle.fill;

    final northPath = Path()
      ..moveTo(0, -radarRadius + 2)
      ..lineTo(-5, -radarRadius + 12)
      ..lineTo(5, -radarRadius + 12)
      ..close();
    canvas.drawPath(northPath, northPaint);


    canvas.restore();

    // 4. Center Player Dot
    final playerPaint = Paint()
      ..color = const Color(0xFF00E5FF)
      ..style = PaintingStyle.fill;
    canvas.drawCircle(center, 3.5, playerPaint);

    // 5. Radar Blips
    for (final blip in blips) {
      final dist = math.sqrt(blip.relativeX * blip.relativeX + blip.relativeZ * blip.relativeZ);
      final normalizedDist = (dist / maxRangeMeters).clamp(0.0, 1.0);
      final blipRadarDist = normalizedDist * (radarRadius - 6.0);

      // Blip relative angle
      final angle = math.atan2(blip.relativeX, -blip.relativeZ) - headingRad;

      final blipOffset = Offset(
        center.dx + blipRadarDist * math.sin(angle),
        center.dy - blipRadarDist * math.cos(angle),
      );

      final blipPaint = Paint()
        ..color = blip.isHostile ? const Color(0xFFFF1744) : const Color(0xFF00E676)
        ..style = PaintingStyle.fill;

      if (blip.isHostile) {
        // Red Diamond for hostile
        final diamond = Path()
          ..moveTo(blipOffset.dx, blipOffset.dy - 4)
          ..lineTo(blipOffset.dx + 4, blipOffset.dy)
          ..lineTo(blipOffset.dx, blipOffset.dy + 4)
          ..lineTo(blipOffset.dx - 4, blipOffset.dy)
          ..close();
        canvas.drawPath(diamond, blipPaint);
      } else {
        // Green circle for friendly
        canvas.drawCircle(blipOffset, 3.0, blipPaint);
      }
    }
  }

  @override
  bool shouldRepaint(covariant _RadarPainter oldDelegate) {
    return oldDelegate.headingDegrees != headingDegrees ||
        oldDelegate.blips != blips ||
        oldDelegate.radarRadius != radarRadius;
  }
}
