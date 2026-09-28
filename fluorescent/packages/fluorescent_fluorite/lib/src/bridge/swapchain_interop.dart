import 'package:flutter/widgets.dart';

/// Zero-Copy Swapchain Interop for Vulkan/VkImage external textures.
///
/// Coordinates frame presentation with native GPU timeline semaphores without CPU stalls.
class FluorescentTextureView extends StatelessWidget {
  /// The native external texture ID registered by Vulkan/Metal backend.
  final int textureId;
  final double width;
  final double height;
  final bool filterQualityHigh;

  const FluorescentTextureView({
    super.key,
    required this.textureId,
    required this.width,
    required this.height,
    this.filterQualityHigh = true,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      height: height,
      child: Texture(
        textureId: textureId,
        filterQuality: filterQualityHigh ? FilterQuality.high : FilterQuality.low,
      ),
    );
  }
}

/// GPU Timeline Semaphore State.
class TimelineSemaphoreTracker {
  int _lastCompletedTimelineValue = 0;

  int get lastCompletedTimelineValue => _lastCompletedTimelineValue;

  /// Updates the latest GPU signaled timeline value.
  void onGpuSignal(int signaledValue) {
    if (signaledValue > _lastCompletedTimelineValue) {
      _lastCompletedTimelineValue = signaledValue;
    }
  }

  /// Checks if a frame rendered under [targetValue] is ready without blocking the UI thread.
  bool isFrameReady(int targetValue) {
    return _lastCompletedTimelineValue >= targetValue;
  }
}
