import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:fluorite_editor/src/node_editor/node_editor_widget.dart';

void main() {
  testWidgets('NodeEditorScreen builds and shows add menu',
      (WidgetTester tester) async {
    await tester.pumpWidget(const MaterialApp(home: NodeEditorScreen()));

    // Verify initial nodes
    expect(find.text('Compute Particle Emitter'), findsWidgets);
    expect(find.text('Half Precision VFX'), findsWidgets);
    expect(find.text('Mobile Optimized Terrain PCG'), findsWidgets);

    // Open add menu
    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();

    expect(find.text('Add Compute Particle Emitter'), findsOneWidget);
  });
}
