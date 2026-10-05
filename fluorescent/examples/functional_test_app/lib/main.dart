import 'dart:async' as dart_async;
import 'package:flutter/material.dart';
import 'package:fluorescent_core/fluorescent_core.dart';
import 'package:flame/game.dart';
import 'package:flame/components.dart';
import 'package:fluorescent_flame/fluorescent_flame.dart';
import 'package:geolocator/geolocator.dart';
import 'package:url_launcher/url_launcher.dart';
import 'engine_bridge.dart';
import 'main_nav.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await FluoriteEngineBridge().startEngine(config: null);
  runApp(const FunctionalTestApp());
}

class FunctionalTestApp extends StatelessWidget {
  const FunctionalTestApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Fluorescent Functional Test',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
        useMaterial3: true,
      ),
      home: const MainNavigationScreen(),
    );
  }
}

class MapScreen extends StatefulWidget {
  const MapScreen({super.key});

  @override
  State<MapScreen> createState() => _MapScreenState();
}

class _MapScreenState extends State<MapScreen> {
  late final CyanMapGame game;
  bool isPhantom = true;
  double _lat = 37.4220;
  double _lng = -122.0841;
  dart_async.StreamSubscription<Position>? _positionStream;

  @override
  void initState() {
    super.initState();
    game = CyanMapGame();
    _initLocation();
  }

  @override
  void dispose() {
    _positionStream?.cancel();
    super.dispose();
  }

  Future<void> _initLocation() async {
    bool serviceEnabled;
    LocationPermission permission;

    serviceEnabled = await Geolocator.isLocationServiceEnabled();
    if (!serviceEnabled) {
      return;
    }

    permission = await Geolocator.checkPermission();
    if (permission == LocationPermission.denied) {
      permission = await Geolocator.requestPermission();
      if (permission == LocationPermission.denied) {
        return;
      }
    }

    if (permission == LocationPermission.deniedForever) {
      return;
    }

    // Get initial position
    try {
      final position = await Geolocator.getCurrentPosition();
      if (mounted) {
        setState(() {
          _lat = position.latitude;
          _lng = position.longitude;
          isPhantom = false;
        });
      }
    } catch (_) {
      // Fallback to default
    }

    // Listen for updates
    const LocationSettings locationSettings = LocationSettings(
      accuracy: LocationAccuracy.high,
      distanceFilter: 10, // update every 10 meters
    );

    _positionStream = Geolocator.getPositionStream(locationSettings: locationSettings).listen(
      (Position position) {
        if (!mounted) return;
        setState(() {
          _lat = position.latitude;
          _lng = position.longitude;
          isPhantom = false;
        });

        // Send telemetry to the Cyan Network for multiplayer syncing
        final engine = FluoriteEngineBridge();
        engine.sendTelemetry("Player Location: [$_lat, $_lng]");
      },
    );
  }

  void _shareLocation() async {
    final url = Uri.parse('https://www.google.com/maps/search/?api=1&query=$_lat,$_lng');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    }
  }

  void _getDirections() async {
    final url = Uri.parse('https://www.google.com/maps/dir/?api=1&destination=$_lat,$_lng');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    }
  }

  void _searchTarget(String target) async {
    final url = Uri.parse('https://www.google.com/maps/search/?api=1&query=$target');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    }
  }

  void _showSearchDialog() {
    final TextEditingController controller = TextEditingController();
    showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: const Text("Search Target"),
          content: TextField(
            controller: controller,
            decoration: const InputDecoration(hintText: "Enter location or coordinates"),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text("Cancel"),
            ),
            TextButton(
              onPressed: () {
                if (controller.text.isNotEmpty) {
                  _searchTarget(controller.text);
                }
                Navigator.pop(context);
              },
              child: const Text("Search"),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Fluorescent Map Field'),
      ),
      body: Stack(
        children: [
          // The Fluorescent 3D Globe is the only visual map rendered.
          GameWidget(
            game: game,
            overlayBuilderMap: {
              'TextureOverlay': (BuildContext context, CyanMapGame game) {
                return const FluorescentTextureOverlay(
                  textureId: 1,
                  width: double.infinity,
                  height: double.infinity,
                );
              },
            },
            initialActiveOverlays: const ['TextureOverlay'],
          ),

          // UI Overlays for logistics and GPS details
          Positioned(
            top: 20,
            left: 20,
            right: 20,
            child: Card(
              color: Colors.white.withValues(alpha: 0.8),
              child: Padding(
                padding: const EdgeInsets.all(12.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Fluorescent Unified Map',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      isPhantom
                          ? 'Status: Phantom Location (Fallback)'
                          : 'Status: Live GPS Tracking',
                      style: TextStyle(
                        color: isPhantom ? Colors.red : Colors.green,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    Text('Location: ${_lat.toStringAsFixed(4)}, ${_lng.toStringAsFixed(4)}'),
                  ],
                ),
              ),
            ),
          ),

          // Action Buttons
          Positioned(
            bottom: 20,
            left: 20,
            right: 20,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                ElevatedButton.icon(
                  onPressed: _shareLocation,
                  icon: const Icon(Icons.share),
                  label: const Text('Share'),
                ),
                ElevatedButton.icon(
                  onPressed: _getDirections,
                  icon: const Icon(Icons.directions),
                  label: const Text('Directions'),
                ),
                ElevatedButton.icon(
                  onPressed: _showSearchDialog,
                  icon: const Icon(Icons.search),
                  label: const Text('Search'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class CyanMapGame extends FlameGame {
  late ThirdPersonCamera camera3d;

  @override
  Future<void> onLoad() async {
    // Load a 3D world that will serve as the globe / environment
    final world3d = await World3D.load('assets/worlds/demo.fworld');

    // Setup 3D camera
    camera3d = ThirdPersonCamera(fov: 60, near: 0.1, far: 1000);

    add(FluorescentViewport(
      world: world3d,
      camera: camera3d,
      textureId: 1,
      position: Vector2(0, 0),
      size: Vector2(400, 800), // Size is controlled by overlay, but provide default here
    ));

    // Add HUD or player markers here inside Flame
    add(
      TextComponent(
        text: 'Fluorescent Engine - Live Multiplayer Field',
        position: Vector2(10, 10),
      ),
    );
  }
}
