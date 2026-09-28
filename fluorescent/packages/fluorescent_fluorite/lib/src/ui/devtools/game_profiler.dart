import 'package:flutter/widgets.dart';

/// In-game real-time telemetry, profiler, and entity debugger overlay.
class InGameDevToolsHud extends StatelessWidget {
  final double currentFps;
  final double frameTimeMs;
  final int drawCallCount;
  final double vramUsedMb;
  final double vramBudgetMb;
  final int activeEntityCount;
  final List<double> recentFrameTimes;
  final VoidCallback? onTriggerShaderHotReload;

  const InGameDevToolsHud({
    super.key,
    required this.currentFps,
    required this.frameTimeMs,
    required this.drawCallCount,
    required this.vramUsedMb,
    required this.vramBudgetMb,
    required this.activeEntityCount,
    required this.recentFrameTimes,
    this.onTriggerShaderHotReload,
  });

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topLeft,
      child: Container(
        margin: const EdgeInsets.all(16),
        padding: const EdgeInsets.all(12),
        width: 320,
        decoration: BoxDecoration(
          color: const Color(0xEE0A0F1D),
          border: Border.all(color: const Color(0xFF00E5FF), width: 1.5),
          borderRadius: BorderRadius.circular(6),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Expanded(
                  child: Text(
                    'FLUORITE PROFILER',
                    style: TextStyle(
                      color: Color(0xFF00E5FF),
                      fontSize: 12,
                      fontWeight: FontWeight.w900,
                      letterSpacing: 1.2,
                    ),
                  ),
                ),
                Text(
                  '${currentFps.toStringAsFixed(1)} FPS',
                  style: TextStyle(
                    color: currentFps >= 55 ? const Color(0xFF00E676) : const Color(0xFFFF1744),
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],

            ),
            const SizedBox(height: 8),

            // Frame time sparkline graph
            SizedBox(
              height: 40,
              width: double.infinity,
              child: CustomPaint(
                painter: _SparklinePainter(times: recentFrameTimes),
              ),
            ),
            const SizedBox(height: 8),

            _metricRow('Frame Time', '${frameTimeMs.toStringAsFixed(2)} ms'),
            _metricRow('Draw Calls', drawCallCount.toString()),
            _metricRow('VRAM Usage', '${vramUsedMb.toStringAsFixed(1)} / ${vramBudgetMb.toStringAsFixed(0)} MB'),
            _metricRow('ECS Entities', activeEntityCount.toString()),

            const SizedBox(height: 10),
            GestureDetector(
              onTap: onTriggerShaderHotReload,
              child: Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(vertical: 6),
                decoration: BoxDecoration(
                  color: const Color(0x3300E5FF),
                  border: Border.all(color: const Color(0xFF00E5FF)),
                  borderRadius: BorderRadius.circular(4),
                ),
                alignment: Alignment.center,
                child: const Text(
                  '⚡ HOT RELOAD SHADERS',
                  style: TextStyle(
                    color: Color(0xFF00E5FF),
                    fontSize: 10,
                    fontWeight: FontWeight.bold,
                    letterSpacing: 1.0,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _metricRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(child: Text(label, style: const TextStyle(color: Color(0xAAFFFFFF), fontSize: 11))),
          Text(value, style: const TextStyle(color: Color(0xFFFFFFFF), fontSize: 11, fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }

}

class _SparklinePainter extends CustomPainter {
  final List<double> times;

  _SparklinePainter({required this.times});

  @override
  void paint(Canvas canvas, Size size) {
    if (times.isEmpty) return;

    final bgPaint = Paint()..color = const Color(0x33000000);
    canvas.drawRect(Offset.zero & size, bgPaint);

    final linePaint = Paint()
      ..color = const Color(0xFF00E5FF)
      ..strokeWidth = 1.5
      ..style = PaintingStyle.stroke;

    final path = Path();
    const maxMs = 33.3; // 30 FPS target line

    for (var i = 0; i < times.length; i++) {
      final x = (i / (times.length - 1).clamp(1, 9999)) * size.width;
      final normalizedY = (times[i] / maxMs).clamp(0.0, 1.0);
      final y = size.height - (normalizedY * size.height);

      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }

    canvas.drawPath(path, linePaint);
  }

  @override
  bool shouldRepaint(covariant _SparklinePainter oldDelegate) => true;
}
