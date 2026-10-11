import re

with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'r') as f:
    content = f.read()

flight_method = """
  Future<void> _flyToCinematic(LatLng targetLocation) async {
    if (_mapController == null) return;
    _recordInteraction();

    // 1. Initial framing (high altitude)
    await _mapController!.animateCamera(
      CameraUpdate.newCameraPosition(
        CameraPosition(
          target: targetLocation,
          zoom: 3.0,
          tilt: 0.0,
          bearing: _currentBearing,
        )
      ),
      duration: const Duration(seconds: 2),
    );

    await Future.delayed(const Duration(milliseconds: 2100));

    // 2. Cinematic Drop (smooth transition into the atmosphere)
    await _mapController!.animateCamera(
      CameraUpdate.newCameraPosition(
        CameraPosition(
          target: targetLocation,
          zoom: 16.5, // Ground level
          tilt: 80.0, // FPS looking forward
          bearing: _currentBearing, // Maintain heading or set to 0
        )
      ),
      duration: const Duration(seconds: 8), // 8 seconds for the entire drop
    );
  }
"""

if "_flyToCinematic" not in content:
    content = content.replace("  void _startIdleSpin() async {", flight_method + "\n  void _startIdleSpin() async {")
    with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'w') as f:
        f.write(content)
    print("Patched!")
else:
    print("Already patched")
