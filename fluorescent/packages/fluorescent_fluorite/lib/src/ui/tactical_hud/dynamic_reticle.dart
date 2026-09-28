import 'dart:math' as math;
import 'package:flutter/widgets.dart';

/// Dynamic combat reticle with animated target lock-on brackets and ballistic lead indicator.
class DynamicReticle extends StatefulWidget {
  final bool isTargetLocked;
  final double leadOffsetX;
  final double leadOffsetY;
  final double weaponHeat;

  const DynamicReticle({
    super.key,
    required this.isTargetLocked,
    required this.leadOffsetX,
    required this.leadOffsetY,
    required this.weaponHeat,
  });

  @override
  State<DynamicReticle> createState() => _DynamicReticleState();
}

class _DynamicReticleState extends State<DynamicReticle> with SingleTickerProviderStateMixin {
  late AnimationController _animController;

  @override
  void initState() {
    super.initState();
    _animController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1500),
    )..repeat();
  }

  @override
  void dispose() {
    _animController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _animController,
      builder: (context, child) {
        return CustomPaint(
          painter: _ReticlePainter(
            isLocked: widget.isTargetLocked,
            leadOffsetX: widget.leadOffsetX,
            leadOffsetY: widget.leadOffsetY,
            weaponHeat: widget.weaponHeat,
            animationValue: _animController.value,
          ),
          child: const SizedBox.expand(),
        );
      },
    );
  }
}

class _ReticlePainter extends CustomPainter {
  final bool isLocked;
  final double leadOffsetX;
  final double leadOffsetY;
  final double weaponHeat;
  final double animationValue;

  _ReticlePainter({
    required this.isLocked,
    required this.leadOffsetX,
    required this.leadOffsetY,
    required this.weaponHeat,
    required this.animationValue,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width * 0.5, size.height * 0.5);

    // 1. Center Crosshair Dot
    final dotPaint = Paint()
      ..color = isLocked ? const Color(0xFFFF1744) : const Color(0xFF00E5FF)
      ..style = PaintingStyle.fill;
    canvas.drawCircle(center, 2.5, dotPaint);

    // 2. Circular Outer Bracket
    final ringPaint = Paint()
      ..color = (isLocked ? const Color(0xFFFF1744) : const Color(0xFF00E5FF)).withAlpha(160)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5;

    final rotationAngle = animationValue * 2.0 * math.pi;
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(rotationAngle);

    const ringRadius = 24.0;
    // Draw 4 arc quadrants
    for (var i = 0; i < 4; i++) {
      final startAngle = i * (math.pi / 2.0) + 0.15;
      canvas.drawArc(
        Rect.fromCircle(center: Offset.zero, radius: ringRadius),
        startAngle,
        (math.pi / 2.0) - 0.3,
        false,
        ringPaint,
      );
    }
    canvas.restore();

    // 3. Ballistic Lead Indicator Circle
    final leadOffset = Offset(
      center.dx + (leadOffsetX * size.width * 0.2),
      center.dy + (leadOffsetY * size.height * 0.2),
    );

    final leadPaint = Paint()
      ..color = const Color(0xFFFFEA00)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5;

    canvas.drawCircle(leadOffset, 8.0, leadPaint);
    canvas.drawLine(
      Offset(leadOffset.dx - 4, leadOffset.dy),
      Offset(leadOffset.dx + 4, leadOffset.dy),
      leadPaint,
    );
    canvas.drawLine(
      Offset(leadOffset.dx, leadOffset.dy - 4),
      Offset(leadOffset.dx, leadOffset.dy + 4),
      leadPaint,
    );

    // 4. Weapon Heat Arc
    if (weaponHeat > 0.05) {
      final heatPaint = Paint()
        ..color = Color.lerp(const Color(0xFF00E5FF), const Color(0xFFFF1744), weaponHeat)!
        ..style = PaintingStyle.stroke
        ..strokeWidth = 3.0;

      canvas.drawArc(
        Rect.fromCircle(center: center, radius: 36.0),
        -math.pi / 2.0,
        weaponHeat * 2.0 * math.pi,
        false,
        heatPaint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _ReticlePainter oldDelegate) {
    return oldDelegate.animationValue != animationValue ||
        oldDelegate.isLocked != isLocked ||
        oldDelegate.leadOffsetX != leadOffsetX ||
        oldDelegate.leadOffsetY != leadOffsetY ||
        oldDelegate.weaponHeat != weaponHeat;
  }
}
