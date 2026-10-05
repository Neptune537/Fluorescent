import 'dart:async' as dart_async;
import 'dart:math';
import 'package:flutter/material.dart';
import 'package:fluorescent_core/fluorescent_core.dart';
import 'package:flame/game.dart';
import 'package:flame/components.dart';
import 'package:fluorescent_flame/fluorescent_flame.dart';
import 'package:google_maps_flutter/google_maps_flutter.dart';
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
  final Random _random = Random();
  dart_async.Timer? _mockLocationTimer;
  late final CyanMapGame game;

  // Driver Locations
  final Vector3 _driver1 = Vector3(0.0, 0.0, 0.0);
  final Vector3 _driver2 = Vector3(10.0, 0.0, 10.0);

  bool showGoogleMap = false;

  @override
  void initState() {
    super.initState();
    game = CyanMapGame();
    game.onTransitionComplete = () {
      if (!mounted) return;
      setState(() {
        showGoogleMap = true;
      });
    };
    _startMockLocationUpdates();
  }

  @override
  void dispose() {
    _mockLocationTimer?.cancel();
    super.dispose();
  }

  void _startMockLocationUpdates() {
    // Send Cyan telemetry updates at 2 Hz
    _mockLocationTimer = dart_async.Timer.periodic(const Duration(milliseconds: 500), (timer) {
      if (!mounted) return;
      setState(() {
        _driver1.x += (_random.nextDouble() - 0.5) * 0.1;
        _driver1.z += (_random.nextDouble() - 0.5) * 0.1;

        _driver2.x += (_random.nextDouble() - 0.5) * 0.1;
        _driver2.z += (_random.nextDouble() - 0.5) * 0.1;

        // FFI Bridge Check - Send Telemetry to Cyan Network
        final engine = FluoriteEngineBridge();
        engine.sendTelemetry("Driver Location: [${_driver1.x}, ${_driver1.z}]");
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Cyan Map'),
      ),
      body: showGoogleMap
          ? const GoogleMap(
              initialCameraPosition: CameraPosition(
                target: LatLng(37.42796133580664, -122.085749655962),
                zoom: 14.4746,
              ),
            )
          : GameWidget(
              game: game,
              overlayBuilderMap: {
                'TextureOverlay': (BuildContext context, CyanMapGame game) {
                  return const FluorescentTextureOverlay(
                    textureId: 1,
                    width: 300,
                    height: 300,
                  );
                },
              },
              initialActiveOverlays: const ['TextureOverlay'],
            ),
    );
  }
}

class CyanMapGame extends FlameGame {
  late ThirdPersonCamera camera3d;
  bool isTransitioning = false;
  double targetFov = 20.0;
  VoidCallback? onTransitionComplete;

  @override
  Future<void> onLoad() async {
    // Load a 3D world that will serve as the globe / environment
    final world3d = await World3D.load('assets/worlds/demo.fworld');

    // Start with a zoomed-out globe view (large FOV or zoomed out position)
    camera3d = ThirdPersonCamera(fov: 60, near: 0.1, far: 1000);

    add(FluorescentViewport(
      world: world3d,
      camera: camera3d,
      textureId: 1,
      position: Vector2(0, 0),
      size: Vector2(400, 800), // Approximate phone size
    ));

    // Render 2D UI overlays on top of the 3D map
    add(
      TextComponent(
        text: 'Rendering globe showing stars and SpaceX missions...',
        position: Vector2(10, 10),
      ),
    );

    // Simulate clicking to transition to street level
    Future.delayed(const Duration(seconds: 3), () {
      isTransitioning = true;
    });
  }

  @override
  void update(double dt) {
    super.update(dt);
    if (isTransitioning) {
      if (camera3d.fov > targetFov) {
        camera3d.fov -= dt * 10;
      } else {
        isTransitioning = false;
        debugPrint("Rendering street level plot");
        onTransitionComplete?.call();
      }
    }
  }
}
