import 'dart:async';
import 'dart:collection'; // firstOrNull (IterableExtensions vive en dart:collection)
import 'dart:convert';
import 'dart:io';

import '../domain/models.dart';
import 'agent.dart';
import 'windows_user_tweaks.dart';

/// Cliente del servicio de Windows (`agents/windows-service`).
/// Protocolo: una petición JSON por línea y una respuesta JSON por línea.
class WindowsAgent implements OptimizerAgent {
  WindowsAgent({this.pipePath = r'\\.\pipe\optimizador-pro', Set<String>? gameCatalog})
      : _catalog = gameCatalog ?? const {};

  final String pipePath;
  final Set<String> _catalog;
  RandomAccessFile? _pipe;
  Future<void> _queue = Future.value();
  Map<String, dynamic>? _lastSnapshot;
  final Map<int, bool> _gameModeBefore = {};

  Future<RandomAccessFile> _open() async {
    if (_pipe != null) return _pipe!;
    try {
      // FileMode.append abre para lectura y escritura sin truncar (necesario en pipes).
      _pipe = await File(pipePath).open(mode: FileMode.append);
      return _pipe!;
    } on FileSystemException catch (e) {
      throw AgentUnavailable('pipe: ${e.message}');
    }
  }

  /// Serializa las peticiones: el servicio atiende una a la vez.
  Future<Map<String, dynamic>> _call(Map<String, dynamic> request) {
    final done = Completer<Map<String, dynamic>>();
    _queue = _queue.then((_) async {
      try {
        final pipe = await _open();
        await pipe.writeString('${jsonEncode(request)}\n');
        await pipe.flush();
        final bytes = <int>[];
        while (true) {
          final b = await pipe.readByte();
          if (b == -1) throw const AgentUnavailable('el servicio cerró la conexión');
          if (b == 10) break;
          bytes.add(b);
        }
        final res = jsonDecode(utf8.decode(bytes)) as Map<String, dynamic>;
        if (res['status'] != 'ok') throw Exception(res['message']);
        done.complete(res);
      } catch (e, st) {
        try {
          await _pipe?.close();
        } catch (_) {}
        _pipe = null; // se reconecta en la siguiente petición
        done.completeError(e, st);
      }
    });
    return done.future;
  }

  Future<dynamic> _data(Map<String, dynamic> request) async => (await _call(request))['data'];

  @override
  Future<Capabilities> capabilities() async => const Capabilities(elevated: 'ready');

  @override
  Future<ScanResult> scan() async {
    final data = await _data({'cmd': 'scan', 'game_mode_on': WindowsUserTweaks.gameModeEnabled()});
    _lastSnapshot = data['snapshot'] as Map<String, dynamic>;
    return ScanResult.fromJson(data['scan'] as Map<String, dynamic>, _lastSnapshot);
  }

  @override
  Future<ApplyResult> apply(List<Tweak> tweaks, {String label = 'one_click'}) async {
    final local = tweaks.where((t) => t.kind == 'enable_game_mode').toList();
    final remote = tweaks.where((t) => t.kind != 'enable_game_mode' && !t.isGuided).toList();

    final data = await _data({'cmd': 'apply', 'label': label, 'tweaks': remote});
    final session = (data['session'] as num).toInt();
    final report = data['report'] as Map<String, dynamic>;
    final failed = (report['failed'] as List).length;

    // Game Mode vive en HKCU: lo aplica la app en el contexto del usuario.
    if (local.isNotEmpty) {
      _gameModeBefore[session] = WindowsUserTweaks.gameModeEnabled() ?? false;
      WindowsUserTweaks.setGameMode(true);
    }
    final failedKinds = (report['failed'] as List).map((f) => jsonEncode(f[0])).toSet();
    final applied = [...remote.where((t) => !failedKinds.contains(jsonEncode(t))), ...local];
    return ApplyResult(session: session, applied: applied, failed: failed);
  }

  @override
  Future<int> undo(int session) async {
    var restored = (await _data({'cmd': 'undo', 'session': session}) as num).toInt();
    final before = _gameModeBefore.remove(session);
    if (before != null) {
      WindowsUserTweaks.setGameMode(before);
      restored++;
    }
    return restored;
  }

  @override
  Future<bool> openScreen(String screen) async {
    const uris = {
      'storage': 'ms-settings:storagesense',
      'game_dashboard': 'ms-settings:gaming-gamemode',
      'battery_saver': 'ms-settings:batterysaver',
    };
    final uri = uris[screen];
    if (uri == null) return false;
    await Process.run('explorer.exe', [uri]);
    return true;
  }

  @override
  Future<bool> requestElevation() async => true;

  /// Juegos en ejecución: procesos del catálogo o el proceso en primer plano.
  @override
  Future<List<GameInfo>> games() async {
    if (_lastSnapshot == null) await scan();
    final procs = (_lastSnapshot!['processes'] as List).cast<Map<String, dynamic>>();
    return procs
        .where((p) => _catalog.contains((p['name'] as String).toLowerCase()) || p['is_foreground'] == true)
        .where((p) => p['is_system'] != true)
        .map((p) => GameInfo(id: p['name'] as String, label: p['name'] as String, pid: (p['pid'] as num).toInt()))
        .toList();
  }

  @override
  Future<void> gameStart(GameInfo game) async {
    await _data({'cmd': 'game_start', 'pid': game.pid, 'exe': game.id});
  }

  @override
  Future<GameTick> gameTick() async {
    final d = await _data({'cmd': 'game_tick'}) as Map<String, dynamic>;
    final recs = (d['recommendations'] as List).cast<Map<String, dynamic>>();
    final cap = recs.map((r) => r['fps_cap']).whereType<num>().map((n) => n.toInt()).firstOrNull;
    return GameTick(
      stats: d['stats'] == null ? null : FrameStats.fromJson(d['stats'] as Map<String, dynamic>),
      fpsCap: (d['fps_cap'] as num?)?.toInt(),
      thermal: d['thermal'] as String? ?? 'normal',
      capTip: cap,
    );
  }

  @override
  Future<int> gameStop() async => ((await _data({'cmd': 'game_stop'}))['restored'] as num).toInt();
}
