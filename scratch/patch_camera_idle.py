import re

with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'r') as f:
    content = f.read()

old_on_camera_idle = """  void _onCameraIdle() {
    if (_mapController == null || !kIsWeb) return;

    final target = _mapController!.cameraPosition?.target;
    if (target != null && (target.latitude != _mapCenter.latitude || target.longitude != _mapCenter.longitude)) {
      setState(() {
        _mapCenter = target;
      });
    }
  }"""

new_on_camera_idle = """  void _onCameraIdle() {
    if (_mapController == null || !kIsWeb) return;

    final cameraPos = _mapController!.cameraPosition;
    final target = cameraPos?.target;

    // Check if we hit the ground floor and need an FPS lock
    if (cameraPos != null && cameraPos.zoom >= 16.0 && cameraPos.tilt < 70.0) {
      // Force FPS perspective lock
      _mapController!.animateCamera(
        CameraUpdate.newCameraPosition(
          CameraPosition(
            target: target ?? const LatLng(0,0),
            zoom: cameraPos.zoom,
            tilt: 80.0, // Force look-forward
            bearing: cameraPos.bearing,
          )
        ),
        duration: const Duration(milliseconds: 500)
      );
    }

    if (target != null && (target.latitude != _mapCenter.latitude || target.longitude != _mapCenter.longitude)) {
      setState(() {
        _mapCenter = target;
      });
    }
  }"""

if old_on_camera_idle in content:
    content = content.replace(old_on_camera_idle, new_on_camera_idle)
    with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'w') as f:
        f.write(content)
    print("Patched onCameraIdle!")
else:
    print("Not found or already patched")
