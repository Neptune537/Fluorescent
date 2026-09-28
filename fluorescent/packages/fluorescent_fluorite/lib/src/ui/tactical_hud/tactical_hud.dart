import 'package:flutter/widgets.dart';
import '../../bridge/gc_firewall.dart';
import '../../bridge/telemetry_stream.dart';
import 'battle_visor.dart';
import 'dynamic_reticle.dart';
import 'holographic_radar.dart';

/// Full Tactical HUD & Battle Visor Layer.
class TacticalHudOverlay extends StatelessWidget {
  final TelemetrySnapshot telemetry;
  final SanitizedUiSnapshot uiState;
  final VisorFilterMode filterMode;

  const TacticalHudOverlay({
    super.key,
    required this.telemetry,
    required this.uiState,
    this.filterMode = VisorFilterMode.normal,
  });

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        // 1. Curved Helmet Glass & Scanlines Filter
        Positioned.fill(
          child: CustomPaint(
            painter: VisorGlassPainter(
              mode: filterMode,
              scanlinePhase: (telemetry.frameIndex % 60) / 60.0,
            ),
          ),
        ),

        // 2. Central Dynamic Combat Reticle
        Positioned.fill(
          child: Center(
            child: DynamicReticle(
              isTargetLocked: uiState.targetLocked,
              leadOffsetX: uiState.targetLeadX,
              leadOffsetY: uiState.targetLeadY,
              weaponHeat: uiState.weaponHeatPercentage,
            ),
          ),
        ),

        // 3. Top Status Compass & Tactical Navigation
        Positioned(
          top: 32,
          left: 0,
          right: 0,
          child: Center(
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              decoration: BoxDecoration(
                color: const Color(0xCC05101E),
                border: Border.all(color: const Color(0x6600E5FF)),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                'HDG ${uiState.compassHeadingDegrees.toStringAsFixed(0)}° | ALT ${uiState.altitudeMeters.toStringAsFixed(0)}m | SPD ${uiState.speedKmh.toStringAsFixed(0)} KM/H',
                style: const TextStyle(
                  color: Color(0xFF00E5FF),
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                  letterSpacing: 1.5,
                ),
              ),
            ),
          ),
        ),

        // 4. Bottom-Left Health & Shield Vitals
        Positioned(
          bottom: 32,
          left: 32,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _buildStatusBar('SHIELD', telemetry.playerShield, telemetry.playerMaxShield, const Color(0xFF00E5FF)),
              const SizedBox(height: 6),
              _buildStatusBar('HEALTH', telemetry.playerHealth, telemetry.playerMaxHealth, const Color(0xFF00E676)),
              const SizedBox(height: 6),
              _buildStatusBar('ENERGY', telemetry.playerEnergy, telemetry.playerMaxEnergy, const Color(0xFFFFD600)),
            ],
          ),
        ),

        // 5. Bottom-Right Holographic Radar & Minimap
        Positioned(
          bottom: 32,
          right: 32,
          child: HolographicRadar(
            headingDegrees: uiState.compassHeadingDegrees,
            blips: telemetry.radarBlips,
          ),
        ),

        // 6. Ammo Counter Bottom-Center-Right
        Positioned(
          bottom: 40,
          right: 250,
          child: Text(
            '${uiState.currentAmmo} / ${uiState.reserveAmmo}',
            style: const TextStyle(
              color: Color(0xFF00E5FF),
              fontSize: 26,
              fontWeight: FontWeight.w900,
              letterSpacing: 2.0,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildStatusBar(String label, double current, double max, Color barColor) {
    final pct = (current / max.clamp(1.0, 999999.0)).clamp(0.0, 1.0);
    return Row(
      children: [
        SizedBox(
          width: 55,
          child: Text(
            label,
            style: TextStyle(color: barColor, fontSize: 10, fontWeight: FontWeight.bold),
          ),
        ),
        Container(
          width: 160,
          height: 8,
          decoration: BoxDecoration(
            color: const Color(0x66000000),
            border: Border.all(color: barColor.withAlpha(120), width: 1),
          ),
          alignment: Alignment.centerLeft,
          child: FractionallySizedBox(
            widthFactor: pct,
            child: Container(color: barColor),
          ),
        ),
        const SizedBox(width: 8),
        Text(
          current.toInt().toString(),
          style: TextStyle(color: barColor, fontSize: 10, fontWeight: FontWeight.bold),
        ),
      ],
    );
  }
}
