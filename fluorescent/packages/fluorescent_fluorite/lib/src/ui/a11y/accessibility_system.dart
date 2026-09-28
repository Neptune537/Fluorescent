import 'package:flutter/widgets.dart';

/// Color vision deficiency / high-contrast modes.
enum ColorAccessibilityMode {
  standard,
  highContrast,
  protanopia,
  deuteranopia,
  tritanopia,
}

/// Subtitle entry with speaker designation and sound effects cue.
class SubtitleCue {
  final String speaker;
  final String text;
  final String? soundEffectCue; // e.g. "[distant explosions]"
  final double durationSeconds;

  const SubtitleCue({
    required this.speaker,
    required this.text,
    this.soundEffectCue,
    this.durationSeconds = 4.0,
  });
}

/// Accessibility state settings.
class AccessibilityConfig extends ChangeNotifier {
  ColorAccessibilityMode colorMode = ColorAccessibilityMode.standard;
  double uiScale = 1.0;
  bool subtitlesEnabled = true;
  double subtitleFontSize = 16.0;
  bool speakerLabelsEnabled = true;
  bool soundEffectCuesEnabled = true;

  // Custom keybind mapping: ActionName -> Key label
  Map<String, String> keybindMap = {
    'MoveForward': 'W',
    'MoveBackward': 'S',
    'MoveLeft': 'A',
    'MoveRight': 'D',
    'Jump': 'Space',
    'Sprint': 'Left Shift',
    'PrimaryFire': 'Left Click',
    'AimDownSights': 'Right Click',
    'Interact': 'E',
    'Reload': 'R',
  };

  void setColorMode(ColorAccessibilityMode mode) {
    colorMode = mode;
    notifyListeners();
  }

  void setUiScale(double scale) {
    uiScale = scale.clamp(0.75, 2.0);
    notifyListeners();
  }

  void remapKeybind(String action, String key) {
    keybindMap[action] = key;
    notifyListeners();
  }
}

/// Dynamic Subtitle Presenter.
class SubtitlePresenter extends StatelessWidget {
  final SubtitleCue? activeCue;
  final AccessibilityConfig config;

  const SubtitlePresenter({
    super.key,
    required this.activeCue,
    required this.config,
  });

  @override
  Widget build(BuildContext context) {
    if (!config.subtitlesEnabled || activeCue == null) {
      return const SizedBox.shrink();
    }

    final cue = activeCue!;
    return Align(
      alignment: Alignment.bottomCenter,
      child: Container(
        margin: const EdgeInsets.only(bottom: 90),
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
        decoration: BoxDecoration(
          color: const Color(0xCC000000),
          borderRadius: BorderRadius.circular(4),
          border: config.colorMode == ColorAccessibilityMode.highContrast
              ? Border.all(color: const Color(0xFFFFFF00), width: 2)
              : null,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (config.soundEffectCuesEnabled && cue.soundEffectCue != null)
              Text(
                cue.soundEffectCue!,
                style: TextStyle(
                  color: const Color(0xFFFFD600),
                  fontSize: config.subtitleFontSize * 0.85,
                  fontStyle: FontStyle.italic,
                ),
              ),
            RichText(
              textAlign: TextAlign.center,
              text: TextSpan(
                style: TextStyle(
                  color: const Color(0xFFFFFFFF),
                  fontSize: config.subtitleFontSize,
                  fontWeight: FontWeight.w600,
                ),
                children: [
                  if (config.speakerLabelsEnabled) ...[
                    TextSpan(
                      text: '${cue.speaker.toUpperCase()}: ',
                      style: const TextStyle(color: Color(0xFF00E5FF), fontWeight: FontWeight.bold),
                    ),
                  ],
                  TextSpan(text: cue.text),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
