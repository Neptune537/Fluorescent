import 'dart:async';
import 'package:flutter/material.dart';
import 'package:geolocator/geolocator.dart';
import 'engine_bridge.dart';
import 'background_location.dart';

enum ZoneType { event, speedTrap }

class GeofenceZone {
  final String id;
  final double lat;
  final double lng;
  final double radius;
  final ZoneType type;
  bool isInside;

  GeofenceZone({
    required this.id,
    required this.lat,
    required this.lng,
    required this.radius,
    required this.type,
    this.isInside = false,
  });
}

class GeofencingOverlay extends StatefulWidget {
  const GeofencingOverlay({super.key});

  @override
  State<GeofencingOverlay> createState() => _GeofencingOverlayState();
}

class _GeofencingOverlayState extends State<GeofencingOverlay> {
  StreamSubscription<Position>? _positionSubscription;
  Position? _currentPosition;

  final List<GeofenceZone> _zones = [
    GeofenceZone(
      id: "ZoneA",
      lat: 37.4220,
      lng: -122.0841,
      radius: 0.002,
      type: ZoneType.event,
    ),
    GeofenceZone(
      id: "SpeedTrap1",
      lat: 37.4200,
      lng: -122.0800,
      radius: 0.001,
      type: ZoneType.speedTrap,
    ),
  ];

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

    for (var zone in _zones) {
      final double radiusInMeters = zone.radius * 111000.0;
      final double distanceInMeters = Geolocator.distanceBetween(
        position.latitude, position.longitude, zone.lat, zone.lng
      );

      final bool isNowInside = distanceInMeters <= radiusInMeters;

      if (isNowInside && !zone.isInside) {
        bridge.sendTelemetry("GeofenceEvent:Enter:${zone.id}");
      } else if (!isNowInside && zone.isInside) {
        bridge.sendTelemetry("GeofenceEvent:Exit:${zone.id}");
      }

      zone.isInside = isNowInside;

      bridge.checkGeofenceIntersection(
        position.latitude, position.longitude, zone.lat, zone.lng, zone.radius
      );
    }
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
          zones: _zones,
        ),
      ),
    );
  }
}

class _GeofencePainter extends CustomPainter {
  final Position? currentPosition;
  final List<GeofenceZone> zones;

  _GeofencePainter({
    required this.currentPosition,
    required this.zones,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radiusScreen = size.width * 0.3;

    if (currentPosition != null) {
      for (var zone in zones) {
        Color baseColor = zone.type == ZoneType.speedTrap ? Colors.orange : Colors.blue;
        Color zoneColor = zone.isInside ? Colors.red : baseColor;

        final paintZone = Paint()
          ..color = zoneColor.withValues(alpha: 0.3)
          ..style = PaintingStyle.fill;

        final paintZoneBorder = Paint()
          ..color = zoneColor
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2.0;

        final latDiff = currentPosition!.latitude - zone.lat;
        final lngDiff = currentPosition!.longitude - zone.lng;

        // Use base zone radius 0.002 as the normalization factor for visual rendering
        final zoneOffset = Offset(
          center.dx - (lngDiff / 0.002) * radiusScreen,
          center.dy + (latDiff / 0.002) * radiusScreen,
        );

        final zoneDrawRadius = (zone.radius / 0.002) * radiusScreen;

        canvas.drawCircle(zoneOffset, zoneDrawRadius, paintZone);
        canvas.drawCircle(zoneOffset, zoneDrawRadius, paintZoneBorder);
      }

      final paintUser = Paint()
        ..color = Colors.green
        ..style = PaintingStyle.fill;

      canvas.drawCircle(center, 10.0, paintUser);
    } else {
      // Just draw the zones at the center if no user position
      for (var zone in zones) {
        Color baseColor = zone.type == ZoneType.speedTrap ? Colors.orange : Colors.blue;

        final paintZone = Paint()
          ..color = baseColor.withValues(alpha: 0.3)
          ..style = PaintingStyle.fill;

        final paintZoneBorder = Paint()
          ..color = baseColor
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2.0;

        final zoneDrawRadius = (zone.radius / 0.002) * radiusScreen;
        canvas.drawCircle(center, zoneDrawRadius, paintZone);
        canvas.drawCircle(center, zoneDrawRadius, paintZoneBorder);
      }
    }
  }

  @override
  bool shouldRepaint(covariant _GeofencePainter oldDelegate) {
    return currentPosition != oldDelegate.currentPosition;
  }
}
