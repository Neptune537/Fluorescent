import 'dart:async';
import 'package:flutter/material.dart';
import '../rust/api/engine.dart';

class StreamingProfilerOverlay extends StatefulWidget {
  const StreamingProfilerOverlay({super.key});

  @override
  State<StreamingProfilerOverlay> createState() => _StreamingProfilerOverlayState();
}

class _StreamingProfilerOverlayState extends State<StreamingProfilerOverlay> {
  Timer? _timer;
  ChunkStreamingMetricsC? _metrics;
  double _camX = 0;
  double _camZ = 0;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(milliseconds: 100), (timer) {
      _fetchMetrics();
    });
  }

  Future<void> _fetchMetrics() async {
    try {
      _camX += 2.0;
      _camZ += 2.0;
      updateCameraPosition(x: _camX, y: 0.0, z: _camZ);
      final metrics = await getStreamingMetrics();
      if (mounted) {
        setState(() {
          _metrics = metrics;
        });
      }
    } catch (e) {
      // Ignore errors if engine isn't ready
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_metrics == null) return const SizedBox.shrink();

    return Positioned(
      top: 16,
      right: 16,
      child: Container(
        width: 320,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Colors.black.withOpacity(0.8),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: Colors.greenAccent, width: 1),
          boxShadow: [
            BoxShadow(
              color: Colors.greenAccent.withOpacity(0.2),
              blurRadius: 8,
              spreadRadius: 2,
            )
          ],
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            const Row(
              children: [
                Icon(Icons.public, color: Colors.greenAccent),
                SizedBox(width: 8),
                Text(
                  'GLOBE STREAMING PROFILER',
                  style: TextStyle(
                    color: Colors.greenAccent,
                    fontWeight: FontWeight.bold,
                    letterSpacing: 1.2,
                  ),
                ),
              ],
            ),
            const Divider(color: Colors.white24),
            _buildStatRow('Resident Chunks', '${_metrics!.residentChunks} / 64'),
            _buildStatRow('VRAM Usage', '${_metrics!.residentMemoryMb.toStringAsFixed(1)} MB'),
            _buildStatRow('Available Allocators', '${_metrics!.availablePoolChunks}'),
            _buildStatRow('GPU Page Faults', '${_metrics!.totalPageFaults}', color: _metrics!.totalPageFaults > 0 ? Colors.redAccent : Colors.white),
            const SizedBox(height: 8),
            const Text(
              'Simulated Camera Coordinates:',
              style: TextStyle(color: Colors.white54, fontSize: 12),
            ),
            Text(
              'X: ${_camX.toStringAsFixed(1)} | Z: ${_camZ.toStringAsFixed(1)}',
              style: const TextStyle(color: Colors.white, fontFamily: 'monospace'),
            ),
            const SizedBox(height: 12),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text('Debug Page Faults (Red Tint)', style: TextStyle(color: Colors.white70, fontSize: 12)),
                Switch(
                  value: true,
                  activeColor: Colors.redAccent,
                  onChanged: (val) {
                    // Call FFI toggle function here
                  },
                ),
              ],
            ),
            const SizedBox(height: 12),
            LinearProgressIndicator(
              value: ((_metrics!.residentChunks.toInt() / 64.0)).clamp(0.0, 1.0),
              backgroundColor: Colors.white12,
              valueColor: const AlwaysStoppedAnimation<Color>(Colors.greenAccent),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStatRow(String label, String value, {Color color = Colors.white}) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.white70)),
          Text(
            value.replaceAll('', ''),
            style: TextStyle(
              color: color,
              fontWeight: FontWeight.bold,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }
}


