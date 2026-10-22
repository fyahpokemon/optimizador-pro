import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../domain/models.dart';
import '../platform/agent.dart';

/// Se sobreescribe en `main.dart` con el agente de la plataforma.
final agentProvider = Provider<OptimizerAgent>((_) => throw UnimplementedError('agentProvider sin configurar'));

final capabilitiesProvider = FutureProvider<Capabilities>((ref) => ref.watch(agentProvider).capabilities());

/// Fases del botón principal.
enum OptimizePhase { scanning, ready, applying, done }

class HomeState {
  const HomeState({
    required this.phase,
    this.scan,
    this.before,
    this.lastApply,
    this.error,
  });

  final OptimizePhase phase;
  final ScanResult? scan;

  /// Puntuación previa a la última optimización (para "Antes · Ahora").
  final int? before;
  final ApplyResult? lastApply;
  final Object? error;

  HomeState copyWith({OptimizePhase? phase, ScanResult? scan, int? before, ApplyResult? lastApply, Object? error}) =>
      HomeState(
        phase: phase ?? this.phase,
        scan: scan ?? this.scan,
        before: before ?? this.before,
        lastApply: lastApply ?? this.lastApply,
        error: error,
      );
}

class HomeController extends Notifier<HomeState> {
  @override
  HomeState build() {
    Future.microtask(rescan);
    return const HomeState(phase: OptimizePhase.scanning);
  }

  OptimizerAgent get _agent => ref.read(agentProvider);

  Future<void> rescan() async {
    state = state.copyWith(phase: OptimizePhase.scanning);
    try {
      final scan = await _agent.scan();
      state = state.copyWith(phase: OptimizePhase.ready, scan: scan);
    } catch (e) {
      state = state.copyWith(phase: OptimizePhase.ready, error: e);
    }
  }

  /// "Optimizar con 1 clic": aplica lo seguro, vuelve a medir y guarda en el historial.
  Future<void> optimize() async {
    final scan = state.scan;
    if (scan == null || state.phase == OptimizePhase.applying) return;
    state = state.copyWith(phase: OptimizePhase.applying);
    try {
      final result = scan.oneClick.isEmpty
          ? const ApplyResult(session: -1, applied: [], failed: 0)
          : await _agent.apply(scan.oneClick);
      if (result.session >= 0) ref.read(historyProvider.notifier).add(result);
      final after = await _agent.scan();
      state = HomeState(phase: OptimizePhase.done, scan: after, before: scan.score, lastApply: result);
    } catch (e) {
      state = state.copyWith(phase: OptimizePhase.ready, error: e);
    }
  }

  Future<void> undoLast() async {
    final session = state.lastApply?.session;
    if (session == null || session < 0) return;
    await ref.read(historyProvider.notifier).undo(session);
    state = HomeState(phase: OptimizePhase.ready, scan: state.scan);
    await rescan();
  }
}

final homeProvider = NotifierProvider<HomeController, HomeState>(HomeController.new);

class HistoryController extends Notifier<List<HistoryEntry>> {
  @override
  List<HistoryEntry> build() => const [];

  void add(ApplyResult r) =>
      state = [HistoryEntry(session: r.session, when: DateTime.now(), applied: r.applied), ...state];

  Future<void> undo(int session) async {
    await ref.read(agentProvider).undo(session);
    state = [for (final e in state) e.session == session ? e.copyWith(undone: true) : e];
  }
}

final historyProvider = NotifierProvider<HistoryController, List<HistoryEntry>>(HistoryController.new);

/// Modo juego: bucle de 1 s mientras dura la partida.
class GameState {
  const GameState({this.game, this.tick, this.capTip, this.summary});

  final GameInfo? game;
  final GameTick? tick;
  final int? capTip;
  final int? summary; // cambios deshechos al terminar

  bool get running => game != null;
}

class GameController extends Notifier<GameState> {
  Timer? _timer;
  bool _busy = false;

  @override
  GameState build() {
    ref.onDispose(() => _timer?.cancel());
    return const GameState();
  }

  Future<void> start(GameInfo game) async {
    await ref.read(agentProvider).gameStart(game);
    state = GameState(game: game);
    _timer = Timer.periodic(const Duration(seconds: 1), (_) => _step());
  }

  Future<void> _step() async {
    if (_busy) return; // nunca solapar ticks si el agente tarda
    _busy = true;
    try {
      final t = await ref.read(agentProvider).gameTick();
      state = GameState(game: state.game, tick: t, capTip: t.capTip ?? state.capTip);
    } catch (_) {
      // Un tick fallido no detiene la partida; el siguiente lo reintenta.
    } finally {
      _busy = false;
    }
  }

  Future<void> stop() async {
    _timer?.cancel();
    _timer = null;
    final restored = await ref.read(agentProvider).gameStop();
    state = GameState(summary: restored);
  }
}

final gameProvider = NotifierProvider<GameController, GameState>(GameController.new);

final gamesProvider = FutureProvider<List<GameInfo>>((ref) => ref.watch(agentProvider).games());
