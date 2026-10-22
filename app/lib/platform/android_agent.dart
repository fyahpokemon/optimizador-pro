import 'dart:convert';

import 'package:flutter/services.dart';

import '../domain/models.dart';
import 'agent.dart';
import 'core_api.dart';
import 'mobile_extras.dart';

/// Agente Android.
///
/// - El núcleo en Rust ([CoreApi]) diagnostica el escaneo.
/// - El plugin Kotlin (`agents/android-module`) mide, aplica y deshace.
/// - La partida corre en nativo dentro de un servicio en primer plano: mientras juegas, la
///   UI de Flutter está en segundo plano y su motor puede quedar en pausa.
class AndroidAgent implements OptimizerAgent, MobileExtras {
  AndroidAgent(this.core);

  final CoreApi core;
  static const _ch = MethodChannel('optimizador/android');
  static const _sessionEvents = EventChannel('optimizador/android/session');
  static const thermalEvents = EventChannel('optimizador/android/thermal');

  bool _elevated = false;

  /// Envía al agente el catálogo de juegos y las listas de procesos (se guardan en disco
  /// para que el modo automático funcione aunque la app no esté abierta).
  Future<void> configure({
    required Set<String> catalog,
    required Set<String> whitelist,
    required Set<String> knownHeavy,
  }) =>
      _invoke<String>('configure', {
        'catalog': catalog.toList(),
        'whitelist': whitelist.toList(),
        'known_heavy': knownHeavy.toList(),
      });

  Future<T> _invoke<T>(String method, [Map<String, dynamic>? args]) async {
    try {
      return (await _ch.invokeMethod<T>(method, args)) as T;
    } on MissingPluginException catch (e) {
      throw AgentUnavailable(e.toString());
    }
  }

  Future<Map<String, dynamic>> _json(String method, [Map<String, dynamic>? args]) async =>
      jsonDecode(await _invoke<String>(method, args)) as Map<String, dynamic>;

  // ---------------------------------------------------------------- OptimizerAgent

  @override
  Future<Capabilities> capabilities() async {
    final j = await _json('capabilities');
    _elevated = j['elevated'] == 'ready';
    return Capabilities(
      elevated: j['elevated'] as String,
      usageAccess: j['usage_access'] as bool,
      measuresFps: _elevated,
    );
  }

  @override
  Future<ScanResult> scan() async {
    final snapshot = await _invoke<String>('snapshot');
    final scan = jsonDecode(await core.scan(snapshot)) as Map<String, dynamic>;
    final snap = jsonDecode(snapshot) as Map<String, dynamic>;
    final result = ScanResult.fromJson(scan, snap);
    if (_elevated) return result;
    // Sin modo avanzado no se puede frenar otras apps: el 1 clic solo limpia caché.
    return ScanResult(
      score: result.score,
      labelKey: result.labelKey,
      headlineKey: result.headlineKey,
      issues: result.issues,
      oneClick: result.oneClick.where((t) => t.kind == 'clean_temp').toList(),
      guided: result.guided,
      tier: result.tier,
      processes: result.processes,
      thermal: result.thermal,
      ramUsedPct: result.ramUsedPct,
    );
  }

  @override
  Future<ApplyResult> apply(List<Tweak> tweaks, {String label = 'one_click'}) async {
    final todo = tweaks.where((t) => !t.isGuided).toList();
    final r = await _json('apply', {'label': label, 'tweaks': jsonEncode(todo)});
    final failed = (r['failed'] as List).cast<Map<String, dynamic>>();
    final failedJson = failed.map((f) => jsonEncode(f['tweak'])).toSet();
    return ApplyResult(
      session: (r['session'] as num).toInt(),
      applied: todo.where((t) => !failedJson.contains(jsonEncode(t))).toList(),
      failed: failed.length,
    );
  }

  @override
  Future<int> undo(int session) async => (await _invoke<num>('undo', {'session': session})).toInt();

  @override
  Future<bool> openScreen(String screen) => _invoke<bool>('openScreen', {'screen': screen});

  @override
  Future<bool> requestElevation() async {
    final ok = await _invoke<bool>('requestElevation');
    if (ok) await capabilities();
    return ok;
  }

  @override
  Future<List<GameInfo>> games() async => (await mobileGames())
      .where((g) => g.enabled)
      .map((g) => GameInfo(id: g.package, label: g.label))
      .toList();

  @override
  Future<void> gameStart(GameInfo game) => _invoke<String>('sessionStart', {'package': game.id, 'label': game.label});

  @override
  Future<GameTick> gameTick() async {
    final s = SessionState.fromJson(await _json('sessionState'));
    return GameTick(stats: s.stats, fpsCap: s.fpsCap, thermal: s.thermal, capTip: s.capTip);
  }

  @override
  Future<int> gameStop() async => (await _invoke<num>('sessionStop')).toInt();

  // ---------------------------------------------------------------- MobileExtras

  @override
  Future<BatteryStatus> batteryStatus() async => BatteryStatus.fromJson(await _json('batteryStatus'));

  @override
  Future<bool> autoGameMode() async => (await _json('settings'))['auto_game_mode'] as bool;

  @override
  Future<void> setAutoGameMode(bool enabled) => _invoke<String>('setAutoGameMode', {'enabled': enabled});

  @override
  Future<List<MobileGame>> mobileGames() async {
    final list = jsonDecode(await _invoke<String>('games')) as List;
    return list.cast<Map<String, dynamic>>().map(MobileGame.fromJson).toList();
  }

  @override
  Future<void> setGameEnabled(String package, bool enabled) =>
      _invoke<String>('setGameEnabled', {'package': package, 'enabled': enabled});

  @override
  Stream<SessionState> sessionStates() => _sessionEvents
      .receiveBroadcastStream()
      .map((e) => SessionState.fromJson(jsonDecode(e as String) as Map<String, dynamic>));
}
