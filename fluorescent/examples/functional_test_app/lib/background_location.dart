import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:geolocator/geolocator.dart';
import 'engine_bridge.dart';

class BackgroundLocationService {
  static final BackgroundLocationService _instance = BackgroundLocationService._internal();

  factory BackgroundLocationService() {
    return _instance;
  }

  BackgroundLocationService._internal();

  StreamSubscription<Position>? _positionStream;
  Position? lastPosition;

  // Expose stream for listeners like GeofencingOverlay
  final StreamController<Position> _positionController = StreamController<Position>.broadcast();
  Stream<Position> get positionStream => _positionController.stream;

  Future<void> startService() async {
    bool serviceEnabled;
    LocationPermission permission;

    serviceEnabled = await Geolocator.isLocationServiceEnabled();
    if (!serviceEnabled) {
      debugPrint("BackgroundLocationService: Location services are disabled.");
      return;
    }

    permission = await Geolocator.checkPermission();
    if (permission == LocationPermission.denied) {
      permission = await Geolocator.requestPermission();
      if (permission == LocationPermission.denied) {
        debugPrint("BackgroundLocationService: Location permissions are denied");
        return;
      }
    }

    if (permission == LocationPermission.deniedForever) {
      debugPrint("BackgroundLocationService: Location permissions are permanently denied.");
      return;
    }

    // To meet the requirement of waking up the app in the background and performing adaptive sampling,
    // a production solution would invoke native Android/iOS APIs (e.g. WorkManager or a foreground service via MethodChannel)
    // or use a package like flutter_background_geolocation.
    // For this implementation, we configure Android settings to run as a foreground service if possible,
    // which keeps the location stream alive in the background on Android.
    final LocationSettings locationSettings = defaultTargetPlatform == TargetPlatform.android
        ? AndroidSettings(
            accuracy: LocationAccuracy.high,
            distanceFilter: 10,
            forceLocationManager: true,
            intervalDuration: const Duration(seconds: 10),
            // The following requires foreground service permissions in AndroidManifest.xml
            foregroundNotificationConfig: const ForegroundNotificationConfig(
              notificationText: "Fluorescent Engine is tracking location in background",
              notificationTitle: "Running in Background",
              enableWakeLock: true,
            ),
          )
        : AppleSettings(allowBackgroundLocationUpdates: true,
            accuracy: LocationAccuracy.high,
            distanceFilter: 10,
          );

    _positionStream ??= Geolocator.getPositionStream(locationSettings: locationSettings).listen(
      (Position position) {
        lastPosition = position;
        _positionController.add(position);
        FluoriteEngineBridge().sendTelemetry("LocationUpdate:${position.latitude},${position.longitude}");
      },
      onError: (e) {
        debugPrint("BackgroundLocationService error: $e");
      }
    );

    debugPrint("BackgroundLocationService: Started (with Background/Foreground support)");
  }

  void stopService() {
    _positionStream?.cancel();
    _positionStream = null;
    debugPrint("BackgroundLocationService: Stopped");
  }
}
