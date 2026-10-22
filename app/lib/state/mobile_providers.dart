import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../platform/mobile_extras.dart';
import 'providers.dart';

/// El agente como [MobileExtras], o null en escritorio (no hay pestañas de Juegos/Batería).
final mobileExtrasProvider = Provider<MobileExtras?>((ref) {
  final agent = ref.watch(agentProvider);
  // Cast explícito: la condicional sola se infiere como OptimizerAgent?, no MobileExtras?.
  return agent is MobileExtras ? agent as MobileExtras : null;
});

/// En `build` y en providers: watch. En métodos de notifiers: read (regla de Riverpod).
MobileExtras _extras(Ref ref) => ref.watch(mobileExtrasProvider)!;
MobileExtras _read(Ref ref) => ref.read(mobileExtrasProvider)!;

/// Se vuelve a leer al volver de Ajustes (ver BatteryScreen).
final batteryProvider = FutureProvider.autoDispose<BatteryStatus>((ref) => _extras(ref).batteryStatus());

class AutoModeController extends AsyncNotifier<bool> {
  @override
  Future<bool> build() => _extras(ref).autoGameMode();

  Future<void> set(bool enabled) async {
    final previous = state;
    state = AsyncData(enabled); // respuesta inmediata en el interruptor
    try {
      await _read(ref).setAutoGameMode(enabled);
    } catch (e, st) {
      state = previous;
      Error.throwWithStackTrace(e, st);
    }
  }
}

final autoModeProvider = AsyncNotifierProvider<AutoModeController, bool>(AutoModeController.new);

class MobileGamesController extends AsyncNotifier<List<MobileGame>> {
  @override
  Future<List<MobileGame>> build() => _extras(ref).mobileGames();

  Future<void> setEnabled(String package, bool enabled) async {
    final list = state.valueOrNull ?? const [];
    state = AsyncData([
      for (final g in list) g.package == package ? MobileGame(package: g.package, label: g.label, enabled: enabled) : g,
    ]);
    await _read(ref).setGameEnabled(package, enabled);
  }
}

final mobileGamesProvider =
    AsyncNotifierProvider<MobileGamesController, List<MobileGame>>(MobileGamesController.new);

/// Partida en vivo (nativa en Android): se actualiza cada segundo mientras se juega.
final sessionProvider = StreamProvider<SessionState>((ref) => _extras(ref).sessionStates());
