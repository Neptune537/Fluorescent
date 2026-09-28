import 'package:flutter/services.dart';

/// Supported discrete button inputs for rapid serialization.
enum GameButton {
  actionA,
  actionB,
  actionX,
  actionY,
  leftBumper,
  rightBumper,
  sprint,
  crouch,
  jump,
  reload,
  interact,
  primaryFire,
  secondaryFire,
}

/// Marshaled input packet dispatched to the native Fluorite simulation tick queue.
class InputCommandPacket {
  final int sequenceNumber;
  final int timestampUs;
  final double moveStickX;
  final double moveStickY;
  final double lookDeltaX;
  final double lookDeltaY;
  final int buttonsBitmask;

  const InputCommandPacket({
    required this.sequenceNumber,
    required this.timestampUs,
    required this.moveStickX,
    required this.moveStickY,
    required this.lookDeltaX,
    required this.lookDeltaY,
    required this.buttonsBitmask,
  });

  bool isButtonPressed(GameButton button) {
    return (buttonsBitmask & (1 << button.index)) != 0;
  }
}

/// Ultra-low latency input collector and marshaler.
class InputCommandMarshaler {
  int _sequenceCounter = 0;
  int _activeButtonsMask = 0;
  double _moveX = 0.0;
  double _moveY = 0.0;
  double _accumulatedLookX = 0.0;
  double _accumulatedLookY = 0.0;

  void onKeyDown(LogicalKeyboardKey key) {
    if (key == LogicalKeyboardKey.keyW) _moveY = 1.0;
    if (key == LogicalKeyboardKey.keyS) _moveY = -1.0;
    if (key == LogicalKeyboardKey.keyA) _moveX = -1.0;
    if (key == LogicalKeyboardKey.keyD) _moveX = 1.0;
    if (key == LogicalKeyboardKey.space) _setButton(GameButton.jump, true);
    if (key == LogicalKeyboardKey.shiftLeft) _setButton(GameButton.sprint, true);
    if (key == LogicalKeyboardKey.keyR) _setButton(GameButton.reload, true);
    if (key == LogicalKeyboardKey.keyE) _setButton(GameButton.interact, true);
  }

  void onKeyUp(LogicalKeyboardKey key) {
    if (key == LogicalKeyboardKey.keyW && _moveY > 0) _moveY = 0.0;
    if (key == LogicalKeyboardKey.keyS && _moveY < 0) _moveY = 0.0;
    if (key == LogicalKeyboardKey.keyA && _moveX < 0) _moveX = 0.0;
    if (key == LogicalKeyboardKey.keyD && _moveX > 0) _moveX = 0.0;
    if (key == LogicalKeyboardKey.space) _setButton(GameButton.jump, false);
    if (key == LogicalKeyboardKey.shiftLeft) _setButton(GameButton.sprint, false);
    if (key == LogicalKeyboardKey.keyR) _setButton(GameButton.reload, false);
    if (key == LogicalKeyboardKey.keyE) _setButton(GameButton.interact, false);
  }

  void onPointerMoveDelta(double dx, double dy) {
    _accumulatedLookX += dx;
    _accumulatedLookY += dy;
  }

  void onPointerDown(int pointerId) {
    _setButton(GameButton.primaryFire, true);
  }

  void onPointerUp(int pointerId) {
    _setButton(GameButton.primaryFire, false);
  }

  void _setButton(GameButton button, bool pressed) {
    if (pressed) {
      _activeButtonsMask |= (1 << button.index);
    } else {
      _activeButtonsMask &= ~(1 << button.index);
    }
  }

  /// Packages the accumulated input state for the current simulation tick.
  InputCommandPacket drainTickPacket() {
    _sequenceCounter++;
    final packet = InputCommandPacket(
      sequenceNumber: _sequenceCounter,
      timestampUs: DateTime.now().microsecondsSinceEpoch,
      moveStickX: _moveX,
      moveStickY: _moveY,
      lookDeltaX: _accumulatedLookX,
      lookDeltaY: _accumulatedLookY,
      buttonsBitmask: _activeButtonsMask,
    );

    // Reset delta accumulators
    _accumulatedLookX = 0.0;
    _accumulatedLookY = 0.0;

    return packet;
  }
}
