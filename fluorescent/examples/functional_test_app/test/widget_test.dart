import 'package:flutter_test/flutter_test.dart';
import 'package:functional_test_app/main.dart';
import 'package:flame/game.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const FunctionalTestApp());
    expect(find.byType(GameWidget<CyanMapGame>), findsOneWidget);
  });
}
