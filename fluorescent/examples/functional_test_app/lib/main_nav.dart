import 'package:flutter/material.dart';
import 'main.dart'; // To access MapScreen
import 'social_feed.dart';
import 'dms_ui.dart';
import 'engine_bridge.dart';

class MainNavigationScreen extends StatefulWidget {
  const MainNavigationScreen({super.key});

  @override
  State<MainNavigationScreen> createState() => _MainNavigationScreenState();
}

class _MainNavigationScreenState extends State<MainNavigationScreen> {
  int _currentIndex = 0;

  final List<Widget> _screens = [
    const MapScreen(),
    const SocialFeedScreen(),
    const DMsScreen(),
  ];

  @override
  void initState() {
    super.initState();
    // Default to map view, set 120fps for high performance
    FluoriteEngineBridge().setTargetFps(120);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: IndexedStack(
        index: _currentIndex,
        children: _screens,
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _currentIndex,
        onDestinationSelected: (index) {
          setState(() {
            _currentIndex = index;
            // Map index is 0, Feed is 1, DMs is 2
            if (index == 0) {
              FluoriteEngineBridge().setTargetFps(120);
            } else {
              // Thermal Scaling to 15fps when not showing the 3D Map
              FluoriteEngineBridge().setTargetFps(15);
            }
          });
        },
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.map_outlined),
            selectedIcon: Icon(Icons.map),
            label: 'Map',
          ),
          NavigationDestination(
            icon: Icon(Icons.dynamic_feed_outlined),
            selectedIcon: Icon(Icons.dynamic_feed),
            label: 'Feed',
          ),
          NavigationDestination(
            icon: Icon(Icons.message_outlined),
            selectedIcon: Icon(Icons.message),
            label: 'Messages',
          ),
        ],
      ),
    );
  }
}
