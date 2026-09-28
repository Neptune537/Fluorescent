import 'package:flutter/widgets.dart';

/// Form factor profile inferred from viewport aspect ratio and width.
enum FormFactorTier {
  ultrawideDesktop, // 21:9 or 32:9 (width >= 2560)
  standardDesktop,  // 16:9 1080p/1440p/4K
  handheldConsole,  // Steam Deck style 16:10 (width 1280, height 800)
  mobileTablet,     // Tablet / phone form factors
}

/// Adaptive layout container that positions game HUD and interaction anchors
/// dynamically based on hardware aspect ratio and display tier.
class ResponsiveGameViewport extends StatelessWidget {
  final Widget gameSurface;
  final Widget hudOverlay;
  final Widget? devtoolsOverlay;

  const ResponsiveGameViewport({
    super.key,
    required this.gameSurface,
    required this.hudOverlay,
    this.devtoolsOverlay,
  });

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final height = constraints.maxHeight;
        final aspectRatio = width / height.clamp(1.0, 99999.0);

        final tier = _calculateTier(width, aspectRatio);

        // Adjust margin safe areas based on form factor
        final horizontalPadding = switch (tier) {
          FormFactorTier.ultrawideDesktop => (width - (height * (16.0 / 9.0))) * 0.4,
          FormFactorTier.standardDesktop => 0.0,
          FormFactorTier.handheldConsole => 16.0,
          FormFactorTier.mobileTablet => 8.0,
        };

        return Stack(
          children: [
            // 1. Native 3D viewport canvas
            Positioned.fill(child: gameSurface),

            // 2. HUD anchored within responsive margins
            Positioned(
              top: 0,
              bottom: 0,
              left: horizontalPadding,
              right: horizontalPadding,
              child: hudOverlay,
            ),

            // 3. Optional DevTools / Profiler overlay
            if (devtoolsOverlay != null)
              Positioned.fill(child: devtoolsOverlay!),
          ],
        );
      },
    );
  }

  FormFactorTier _calculateTier(double width, double aspectRatio) {
    if (aspectRatio >= 2.0 && width >= 2560) {
      return FormFactorTier.ultrawideDesktop;
    } else if (width >= 1920) {
      return FormFactorTier.standardDesktop;
    } else if (width <= 1400 && aspectRatio >= 1.5) {
      return FormFactorTier.handheldConsole;
    } else {
      return FormFactorTier.mobileTablet;
    }
  }
}
