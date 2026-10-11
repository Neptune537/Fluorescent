import re

with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'r') as f:
    content = f.read()

old_on_camera_move = """  void _onCameraMove() {
    if (_mapController == null || !kIsWeb) return;

    try {
      final zoom = _mapController!.cameraPosition?.zoom ?? 3.0;
      // Use setProjection with transition settings or dynamically switch at threshold
      // MapLibre natively handles the crossfade between projections
      if (zoom < 5.0) {
        _mapController!.setProjection('globe');
      } else {
        _mapController!.setProjection('mercator');
      }
    } catch (_) {
      // Ignore on unsupported platforms
    }
  }"""

new_on_camera_move = """  void _onCameraMove() {
    if (_mapController == null || !kIsWeb) return;

    try {
      final zoom = _mapController!.cameraPosition?.zoom ?? 3.0;

      // Cloud/Fog Layer Transition Logic
      if (zoom < 5.0) {
        // Space View: Wrap globe, clear atmosphere, no fog
        _mapController!.setProjection('globe');
        _mapController!.setFog(
          const FogProperties(
            color: '#FFFFFF',
            highColor: '#245bde',
            spaceColor: '#0b162c',
            range: [0.5, 3.0],
            horizonBlend: 0.1,
          )
        );
      } else if (zoom >= 5.0 && zoom < 12.0) {
        // Cloud Transition: Aggressive fog to hide LOD popping
        _mapController!.setProjection('mercator');

        // Calculate fog density based on depth into the transition layer
        final progress = (zoom - 5.0) / 7.0; // 0.0 to 1.0
        final fogRangeStart = 0.5 - (progress * 0.4); // Bring fog closer
        final fogRangeEnd = 3.0 - (progress * 2.0);

        _mapController!.setFog(
          FogProperties(
            color: '#DDDDDD',
            highColor: '#AAAAAA',
            spaceColor: '#FFFFFF',
            range: [fogRangeStart, fogRangeEnd], // Thicker fog
            horizonBlend: 0.8, // Heavy blend
          )
        );
      } else {
        // Ground Level: Localized street view, thin out fog for visibility
        _mapController!.setProjection('mercator');
        _mapController!.setFog(
          const FogProperties(
            color: '#FFFFFF',
            highColor: '#245bde',
            spaceColor: '#0b162c',
            range: [1.0, 10.0], // Push fog far away
            horizonBlend: 0.1,
          )
        );
      }
    } catch (_) {
      // Ignore on unsupported platforms
    }
  }"""

if old_on_camera_move in content:
    content = content.replace(old_on_camera_move, new_on_camera_move)
    with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'w') as f:
        f.write(content)
    print("Patched onCameraMove!")
else:
    print("Not found or already patched")
