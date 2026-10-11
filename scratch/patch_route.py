import re

with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'r') as f:
    content = f.read()

old_draw_route = """  Future<void> _drawRoute(LatLng start, LatLng end) async {
    if (_mapController == null) return;

    // Using OSRM public API for demo purposes, switched to HTTPS
    final url = 'https://router.project-osrm.org/route/v1/driving/${start.longitude},${start.latitude};${end.longitude},${end.latitude}?geometries=geojson';
    try {
      final response = await http.get(Uri.parse(url));
      if (response.statusCode == 200) {
        final data = json.decode(response.body);
        final coordinates = data['routes'][0]['geometry']['coordinates'] as List;

        final routeGeometry = {
          "type": "Feature",
          "properties": {},
          "geometry": {
            "type": "LineString",
            "coordinates": coordinates
          }
        };

        // Remove old route if exists
        try {
          await _mapController!.removeLayer("route-layer");
          await _mapController!.removeSource("route-source");
        } catch (e) {
          // Ignore if they don't exist
        }

        await _mapController!.addGeoJsonSource("route-source", routeGeometry);
        await _mapController!.addLineLayer("route-source", "route-layer", const LineLayerProperties(
          lineColor: "#00E5FF",
          lineWidth: 4.0,
        ));
      }
    } catch (e) {
      // Error drawing route
    }
  }"""


new_draw_route = """  Future<void> _drawRoute(LatLng start, LatLng end) async {
    if (_mapController == null) return;

    // Using OSRM public API for demo purposes, switched to HTTPS
    final url = 'https://router.project-osrm.org/route/v1/driving/${start.longitude},${start.latitude};${end.longitude},${end.latitude}?geometries=geojson';
    try {
      final response = await http.get(Uri.parse(url));
      if (response.statusCode == 200) {
        final data = json.decode(response.body);
        final coordinates = data['routes'][0]['geometry']['coordinates'] as List;

        final routeGeometry = {
          "type": "Feature",
          "properties": {},
          "geometry": {
            "type": "LineString",
            "coordinates": coordinates
          }
        };

        // Remove old route if exists
        try {
          await _mapController!.removeLayer("route-layer");
          await _mapController!.removeSource("route-source");
        } catch (e) {
          // Ignore if they don't exist
        }

        await _mapController!.addGeoJsonSource("route-source", routeGeometry);
        await _mapController!.addLineLayer("route-source", "route-layer", const LineLayerProperties(
          lineColor: "#00E5FF",
          lineWidth: 4.0,
        ));

        // Start driving simulation
        _simulateDrivingAlongRoute(coordinates);
      }
    } catch (e) {
      // Error drawing route
    }
  }

  Future<void> _simulateDrivingAlongRoute(List coordinates) async {
    if (_mapController == null || coordinates.isEmpty) return;

    // Jump to start of route, FPS view
    await _mapController!.animateCamera(
      CameraUpdate.newCameraPosition(
        CameraPosition(
          target: LatLng(coordinates[0][1], coordinates[0][0]),
          zoom: 18.0,
          tilt: 80.0,
        )
      )
    );

    await Future.delayed(const Duration(seconds: 2));

    // Drive sequence
    for (int i = 1; i < coordinates.length; i++) {
      final point = coordinates[i];
      final target = LatLng(point[1], point[0]); // OSRM returns [lon, lat]

      await _mapController!.animateCamera(
        CameraUpdate.newCameraPosition(
          CameraPosition(
            target: target,
            zoom: 18.0,
            tilt: 80.0,
          )
        ),
        duration: const Duration(seconds: 1)
      );
      await Future.delayed(const Duration(seconds: 1));
    }
  }"""

if old_draw_route in content:
    content = content.replace(old_draw_route, new_draw_route)
    with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'w') as f:
        f.write(content)
    print("Patched route drawing!")
else:
    print("Not found or already patched")
