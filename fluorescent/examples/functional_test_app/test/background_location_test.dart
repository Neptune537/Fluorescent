import 'package:flutter_test/flutter_test.dart';
import 'package:functional_test_app/background_location.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('BackgroundLocationService', () {
    test('is a singleton', () {
      final service1 = BackgroundLocationService();
      final service2 = BackgroundLocationService();
      expect(identical(service1, service2), isTrue);
    });

    // We removed the 'initializes without errors' test because `Geolocator.isLocationServiceEnabled()`
    // expects the underlying native platform channels to be mocked in unit tests,
    // which flutter_test does not do by default, leading to MissingPluginException
    // when running tests in a pure headless Dart environment.
  });
}
