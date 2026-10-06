import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:functional_test_app/geofencing.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('GeofencingOverlay', () {
    testWidgets('renders CustomPaint correctly without throwing', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: GeofencingOverlay(),
          ),
        ),
      );

      // Verify a CustomPaint is rendered for the GeofencingOverlay
      expect(find.byType(CustomPaint), findsWidgets);
    });
  });
}
