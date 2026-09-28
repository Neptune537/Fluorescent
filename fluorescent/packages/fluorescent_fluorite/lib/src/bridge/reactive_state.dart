import 'package:flutter/foundation.dart';
import 'telemetry_stream.dart';
import 'gc_firewall.dart';

/// Dynamic weather and atmospheric conditions state.
class WeatherState {
  final double cloudCoverage; // 0.0 to 1.0
  final double windSpeedKmh;
  final double ambientTemperatureC;
  final bool isRaining;

  const WeatherState({
    required this.cloudCoverage,
    required this.windSpeedKmh,
    required this.ambientTemperatureC,
    required this.isRaining,
  });

  factory WeatherState.defaultOvercast() {
    return const WeatherState(
      cloudCoverage: 0.65,
      windSpeedKmh: 24.0,
      ambientTemperatureC: 18.5,
      isRaining: false,
    );
  }
}

/// Tactical game resources (credits, fuel, energy, nanites).
class ResourceState {
  final int credits;
  final double fuelPercent;
  final int rawMaterials;
  final int techTokens;

  const ResourceState({
    required this.credits,
    required this.fuelPercent,
    required this.rawMaterials,
    required this.techTokens,
  });

  factory ResourceState.defaultPlayer() {
    return const ResourceState(
      credits: 24500,
      fuelPercent: 0.88,
      rawMaterials: 350,
      techTokens: 12,
    );
  }
}

/// Reactive State Hub coordinating unidirectional state updates to UI widgets.
class FluoriteStateHub extends ChangeNotifier {
  TelemetrySnapshot _telemetry = const TelemetrySnapshot(
    frameIndex: 0,
    timestampUs: 0,
    playerHealth: 100.0,
    playerMaxHealth: 100.0,
    playerShield: 100.0,
    playerMaxShield: 100.0,
    playerEnergy: 100.0,
    playerMaxEnergy: 100.0,
    heatLevel: 0,
    radarBlips: [],
  );

  SanitizedUiSnapshot _uiSnapshot = SanitizedUiSnapshot.mock();
  WeatherState _weather = WeatherState.defaultOvercast();
  ResourceState _resources = ResourceState.defaultPlayer();

  TelemetrySnapshot get telemetry => _telemetry;
  SanitizedUiSnapshot get uiSnapshot => _uiSnapshot;
  WeatherState get weather => _weather;
  ResourceState get resources => _resources;

  void updateTelemetry(TelemetrySnapshot next) {
    _telemetry = next;
    notifyListeners();
  }

  void updateUiSnapshot(SanitizedUiSnapshot next) {
    _uiSnapshot = next;
    notifyListeners();
  }

  void updateWeather(WeatherState next) {
    if (_weather.cloudCoverage != next.cloudCoverage || _weather.isRaining != next.isRaining) {
      _weather = next;
      notifyListeners();
    }
  }

  void updateResources(ResourceState next) {
    if (_resources.credits != next.credits || _resources.fuelPercent != next.fuelPercent) {
      _resources = next;
      notifyListeners();
    }
  }
}
