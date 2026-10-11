import re

with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'r') as f:
    content = f.read()

old_on_tap = """                            onTap: () {
                              _recordInteraction();
                              _mapController?.animateCamera(CameraUpdate.newLatLngZoom(
                                LatLng(peer.lat, peer.lon),
                                10.0
                              ));
                            },"""

new_on_tap = """                            onTap: () {
                              _flyToCinematic(LatLng(peer.lat, peer.lon));
                            },"""

if old_on_tap in content:
    content = content.replace(old_on_tap, new_on_tap)
    with open('third_party/tithX/fluoridian_app/lib/screens/constellation_map_page.dart', 'w') as f:
        f.write(content)
    print("Patched onTap!")
else:
    print("Not found or already patched")
