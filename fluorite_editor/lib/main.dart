import 'package:flutter/material.dart';
import 'src/node_editor/node_editor_widget.dart';

import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  print('Init RustLib...'); await RustLib.init(); print('RustLib init done!');
  print('Running app...'); runApp(const FluoriteEditorApp());
}

class FluoriteEditorApp extends StatelessWidget {
  const FluoriteEditorApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Fluorite Editor - Phase 3 Tools',
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF1E1E1E),
        colorScheme: const ColorScheme.dark(
          primary: Colors.blueAccent,
          secondary: Colors.tealAccent,
        ),
      ),
      home: const Scaffold(
        body: NodeEditorScreen(),
      ),
    );
  }
}


