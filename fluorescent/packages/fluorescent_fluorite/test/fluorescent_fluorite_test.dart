import 'dart:typed_data';
import 'package:flutter/widgets.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:fluorescent_fluorite/fluorescent_fluorite.dart';

void main() {
  group('Fluoropod Bridge & State Tests', () {
    test('TimelineSemaphoreTracker tracks GPU timeline signals', () {
      final tracker = TimelineSemaphoreTracker();
      expect(tracker.lastCompletedTimelineValue, 0);
      expect(tracker.isFrameReady(1), isFalse);

      tracker.onGpuSignal(1);
      expect(tracker.lastCompletedTimelineValue, 1);
      expect(tracker.isFrameReady(1), isTrue);
      expect(tracker.isFrameReady(2), isFalse);
    });

    test('SanitizedUiSnapshot parses zero-copy ByteData without entity allocations', () {
      final buffer = ByteData(40);
      buffer.setFloat32(0, 180.0, Endian.little); // heading
      buffer.setFloat32(4, 95.0, Endian.little);  // speed
      buffer.setFloat32(8, 250.0, Endian.little); // altitude
      buffer.setUint32(12, 30, Endian.little);    // current ammo
      buffer.setUint32(16, 90, Endian.little);    // reserve ammo
      buffer.setFloat32(20, 0.4, Endian.little);  // heat
      buffer.setUint32(24, 1, Endian.little);     // target locked
      buffer.setFloat32(28, 0.05, Endian.little); // lead x
      buffer.setFloat32(32, -0.02, Endian.little);// lead y
      buffer.setFloat32(36, 450.0, Endian.little);// quest dist

      final snapshot = SanitizedUiSnapshot.fromByteData(buffer);
      expect(snapshot.compassHeadingDegrees, 180.0);
      expect(snapshot.speedKmh, 95.0);
      expect(snapshot.altitudeMeters, 250.0);
      expect(snapshot.currentAmmo, 30);
      expect(snapshot.reserveAmmo, 90);
      expect(snapshot.targetLocked, isTrue);
      expect(snapshot.activeQuestDistance, 450.0);
    });

    test('InputCommandMarshaler captures input keys and builds packets', () {
      final marshaler = InputCommandMarshaler();
      marshaler.onKeyDown(LogicalKeyboardKey.keyW);
      marshaler.onKeyDown(LogicalKeyboardKey.space);
      marshaler.onPointerMoveDelta(15.0, -5.0);

      final packet = marshaler.drainTickPacket();
      expect(packet.sequenceNumber, 1);
      expect(packet.moveStickY, 1.0);
      expect(packet.lookDeltaX, 15.0);
      expect(packet.lookDeltaY, -5.0);
      expect(packet.isButtonPressed(GameButton.jump), isTrue);

      // Drain resets look deltas
      final nextPacket = marshaler.drainTickPacket();
      expect(nextPacket.sequenceNumber, 2);
      expect(nextPacket.lookDeltaX, 0.0);
    });

    test('FluoriteStateHub updates discrete reactive slices', () {
      final hub = FluoriteStateHub();
      var notificationCount = 0;
      hub.addListener(() => notificationCount++);

      hub.updateWeather(const WeatherState(
        cloudCoverage: 0.9,
        windSpeedKmh: 45.0,
        ambientTemperatureC: 12.0,
        isRaining: true,
      ));

      expect(notificationCount, 1);
      expect(hub.weather.isRaining, isTrue);
      expect(hub.weather.cloudCoverage, 0.9);
    });

    test('AccessibilityConfig manages visual modes and keybind remap', () {
      final config = AccessibilityConfig();
      config.setColorMode(ColorAccessibilityMode.highContrast);
      config.remapKeybind('Jump', 'ButtonA');

      expect(config.colorMode, ColorAccessibilityMode.highContrast);
      expect(config.keybindMap['Jump'], 'ButtonA');
    });
  });

  group('Presentation Engine Widget Tests', () {
    testWidgets('TacticalHudOverlay renders HUD indicators', (tester) async {
      final telemetry = TelemetrySnapshot(
        frameIndex: 10,
        timestampUs: 1000,
        playerHealth: 85.0,
        playerMaxHealth: 100.0,
        playerShield: 50.0,
        playerMaxShield: 100.0,
        playerEnergy: 90.0,
        playerMaxEnergy: 100.0,
        heatLevel: 2,
        radarBlips: const [
          RadarBlipData(
            entityId: 1,
            relativeX: 20.0,
            relativeZ: -50.0,
            altitudeOffset: 0.0,
            isHostile: true,
            blipType: 0,
          ),
        ],
      );

      final uiSnapshot = SanitizedUiSnapshot.mock();

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: SizedBox(
            width: 1280,
            height: 720,
            child: TacticalHudOverlay(
              telemetry: telemetry,
              uiState: uiSnapshot,
            ),
          ),
        ),
      );

      expect(find.textContaining('HDG'), findsOneWidget);
      expect(find.text('SHIELD'), findsOneWidget);
      expect(find.text('HEALTH'), findsOneWidget);
      expect(find.byType(HolographicRadar), findsOneWidget);
    });

    testWidgets('InGameDevToolsHud displays real-time telemetry metrics', (tester) async {
      var reloadTriggered = false;

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: SizedBox(
            width: 1280,
            height: 720,
            child: InGameDevToolsHud(
              currentFps: 60.0,
              frameTimeMs: 16.2,
              drawCallCount: 142,
              vramUsedMb: 2150.0,
              vramBudgetMb: 6144.0,
              activeEntityCount: 8500,
              recentFrameTimes: const [16.2, 16.0, 16.5, 15.9],
              onTriggerShaderHotReload: () => reloadTriggered = true,
            ),
          ),
        ),
      );

      expect(find.text('FLUORITE PROFILER'), findsOneWidget);
      expect(find.text('60.0 FPS'), findsOneWidget);
      expect(find.text('142'), findsOneWidget);

      await tester.tap(find.text('⚡ HOT RELOAD SHADERS'));
      expect(reloadTriggered, isTrue);
    });

    testWidgets('NarrativeDeckView renders codex archival entries', (tester) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: SizedBox(
            width: 1280,
            height: 720,
            child: NarrativeDeckView(
              activeMissionName: 'Project Fluorite Dawn',
              activeMissionObjective: 'Infiltrate the orbital relay station.',
              codexEntries: const [
                CodexEntry(
                  id: 'entry_01',
                  title: 'Fluoridian Core Foundation',
                  faction: 'Frontier Cartel',
                  classification: 'CONFIDENTIAL',
                  excerpt: 'Native 3D real-time geometry architecture powered by Rust & Vulkan.',
                  isUnlocked: true,
                ),
              ],
            ),
          ),
        ),
      );

      expect(find.text('TACTICAL NARRATIVE DECK'), findsOneWidget);
      expect(find.text('Fluoridian Core Foundation'), findsWidgets);
    });

    testWidgets('SubtitlePresenter displays speaker tags and sound cues', (tester) async {
      final config = AccessibilityConfig();

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: SizedBox(
            width: 800,
            height: 600,
            child: SubtitlePresenter(
              config: config,
              activeCue: const SubtitleCue(
                speaker: 'Commander',
                text: 'All units, engage target immediately.',
                soundEffectCue: '[heavy thruster burn]',
              ),
            ),
          ),
        ),
      );

      expect(find.text('[heavy thruster burn]'), findsOneWidget);
      expect(find.byType(RichText), findsWidgets);
    });

  });
}
