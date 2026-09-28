import 'package:flutter/widgets.dart';

/// Visor visual filter mode.
enum VisorFilterMode {
  normal,
  nightVisionPhosphor,
  thermalInfrared,
  tacticalArOverlay,
}

/// Custom painter rendering curved helmet visor glass, scanlines, and vignette aberration.
class VisorGlassPainter extends CustomPainter {
  final VisorFilterMode mode;
  final double scanlinePhase;

  const VisorGlassPainter({
    required this.mode,
    required this.scanlinePhase,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;

    // 1. Visor curvature vignette gradient
    final baseColor = switch (mode) {

      VisorFilterMode.normal => const Color(0x1A00E5FF), // Cyan tactical tint
      VisorFilterMode.nightVisionPhosphor => const Color(0x3300FF66), // Green phosphor
      VisorFilterMode.thermalInfrared => const Color(0x33FF3300), // Thermal amber
      VisorFilterMode.tacticalArOverlay => const Color(0x227C4DFF), // Purple tactical
    };

    final vignettePaint = Paint()
      ..shader = RadialGradient(
        center: Alignment.center,
        radius: 0.85,
        colors: [
          const Color(0x00000000),
          baseColor.withAlpha(20),
          const Color(0xDD050B14),
        ],
        stops: const [0.0, 0.7, 1.0],
      ).createShader(rect);

    canvas.drawRect(rect, vignettePaint);

    // 2. Scanline raster effect
    final scanlinePaint = Paint()
      ..color = const Color(0x1200FFFF)
      ..strokeWidth = 1.0;

    const step = 4.0;
    final offsetY = (scanlinePhase * step) % step;
    for (var y = offsetY; y < size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), scanlinePaint);
    }

    // 3. Curved helmet glass border bevels
    final borderPaint = Paint()
      ..color = baseColor.withAlpha(90)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.5;

    final path = Path()
      ..moveTo(size.width * 0.05, size.height * 0.08)
      ..lineTo(size.width * 0.95, size.height * 0.08)
      ..lineTo(size.width * 0.98, size.height * 0.92)
      ..lineTo(size.width * 0.02, size.height * 0.92)
      ..close();

    canvas.drawPath(path, borderPaint);
  }

  @override
  bool shouldRepaint(covariant VisorGlassPainter oldDelegate) {
    return oldDelegate.mode != mode || oldDelegate.scanlinePhase != scanlinePhase;
  }
}
