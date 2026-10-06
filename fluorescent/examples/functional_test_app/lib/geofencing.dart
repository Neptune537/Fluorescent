import 'dart:async';
import 'package:flutter/material.dart';
import 'package:geolocator/geolocator.dart';
import 'engine_bridge.dart';
import 'background_location.dart';

class GeofencingOverlay extends StatefulWidget {
  const GeofencingOverlay({super.key});

  @override
  State<GeofencingOverlay> createState() => _GeofencingOverlayState();
}

class _GeofencingOverlayState extends State<GeofencingOverlay> {
  StreamSubscription<Position>? _positionSubscription;
  Position? _currentPosition;

  // Dummy geofence zone
  final double zoneLat = 37.4220;
  final double zoneLng = -122.0841;
  final double zoneRadius = 0.002;
  bool _isInsideZone = false;

  @override
  void initState() {
    super.initState();
    final backgroundService = BackgroundLocationService();
    _currentPosition = backgroundService.lastPosition;
    _checkGeofences(_currentPosition);

    _positionSubscription = backgroundService.positionStream.listen((Position position) {
      if (mounted) {
        setState(() {
          _currentPosition = position;
          _checkGeofences(position);
        });
      }
    });
  }

  void _checkGeofences(Position? position) {
    if (position == null) return;

    final bridge = FluoriteEngineBridge();

    // Implement math to detect boundary crossings (simple radius distance)
    // We calculate distance using Geolocator.distanceBetween which gives distance in meters.
    // The dummy zoneRadius is 0.002 roughly degrees. 1 degree is ~111km, so 0.002 degrees is ~222 meters.
    final double radiusInMeters = zoneRadius * 111000.0;
    final double distanceInMeters = Geolocator.distanceBetween(
      position.latitude, position.longitude, zoneLat, zoneLng
    );

    final bool isNowInside = distanceInMeters <= radiusInMeters;

    if (isNowInside && !_isInsideZone) {
      bridge.sendTelemetry("GeofenceEvent:Enter:ZoneA");
    } else if (!isNowInside && _isInsideZone) {
      bridge.sendTelemetry("GeofenceEvent:Exit:ZoneA");
    }

    _isInsideZone = isNowInside;
  }

  @override
  void dispose() {
    _positionSubscription?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: CustomPaint(
        size: Size.infinite,
        painter: _GeofencePainter(
          currentPosition: _currentPosition,
          zoneLat: zoneLat,
          zoneLng: zoneLng,
          zoneRadius: zoneRadius,
          isInsideZone: _isInsideZone,
        ),
      ),
    );
  }
}

class _GeofencePainter extends CustomPainter {
  final Position? currentPosition;
  final double zoneLat;
  final double zoneLng;
  final double zoneRadius;
  final bool isInsideZone;

  _GeofencePainter({
    required this.currentPosition,
    required this.zoneLat,
    required this.zoneLng,
    required this.zoneRadius,
    required this.isInsideZone,
  });

  @override
  void paint(Canvas canvas, Size size) {
    // A simple visual representation of geofencing logic
    // In a real map view, we would project lat/lng to screen coordinates.
    // Here we just draw a static representation of the zone and user.

    final paintZone = Paint()
      ..color = isInsideZone ? Colors.red.withValues(alpha: 0.3) : Colors.blue.withValues(alpha: 0.3)
      ..style = PaintingStyle.fill;

    final paintZoneBorder = Paint()
      ..color = isInsideZone ? Colors.red : Colors.blue
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0;

    // Draw Zone at center of screen for visualization
    final center = Offset(size.width / 2, size.height / 2);
    // arbitrary scale for visualization
    final radiusScreen = size.width * 0.3;

    canvas.drawCircle(center, radiusScreen, paintZone);
    canvas.drawCircle(center, radiusScreen, paintZoneBorder);

    // Draw user location
    if (currentPosition != null) {
      // Very naive mapping of coordinates to screen
      final latDiff = currentPosition!.latitude - zoneLat;
      final lngDiff = currentPosition!.longitude - zoneLng;

      final userOffset = Offset(
        center.dx + (lngDiff / zoneRadius) * radiusScreen,
        center.dy - (latDiff / zoneRadius) * radiusScreen,
      );

      final paintUser = Paint()
        ..color = Colors.green
        ..style = PaintingStyle.fill;

      canvas.drawCircle(userOffset, 10.0, paintUser);
    }
  }

  @override
  bool shouldRepaint(covariant _GeofencePainter oldDelegate) {
    return currentPosition != oldDelegate.currentPosition ||
           isInsideZone != oldDelegate.isInsideZone;
  }
}
