import 'dart:io';

import 'package:win32_registry/win32_registry.dart';

/// Ajustes que viven en el registro del usuario (HKCU) y por tanto los aplica la app, no el
/// servicio (que corre como SYSTEM y vería otro HKCU).
abstract final class WindowsUserTweaks {
  static const _gameBar = r'Software\Microsoft\GameBar';
  static const _value = 'AutoGameModeEnabled';

  /// null si no es Windows o la clave no existe (Windows lo trata como activado).
  static bool? gameModeEnabled() {
    if (!Platform.isWindows) return null;
    try {
      final key = Registry.openPath(RegistryHive.currentUser, path: _gameBar);
      final v = key.getValueAsInt(_value);
      key.close();
      return v == null ? true : v != 0;
    } catch (_) {
      return true;
    }
  }

  static void setGameMode(bool on) {
    if (!Platform.isWindows) return;
    final key = Registry.currentUser.createKey(_gameBar);
    key.createValue(RegistryValue(_value, RegistryValueType.int32, on ? 1 : 0));
    key.close();
  }
}
