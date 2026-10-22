import 'dart:async';
import 'dart:math';

import '../domain/models.dart';
import 'agent.dart';
import 'mobile_extras.dart';

/// Agente simulado de un móvil Android: permite ejecutar la UI sin dispositivo ni núcleo
/// nativo (desarrollo, capturas para la tienda, tests de widgets).
/// `flutter run -t lib/main_demo.dart`
class DemoAgent implements OptimizerAgent, MobileExtras {
  DemoAgent({this.latency = const Duration(milliseconds: 900), int seed = 7}) : _rng = Random(seed);

  final Duration latency;
  final Random _rng;
  bool _optimized = false;
  bool _auto = false;
  int _session = 0;
  int _tick = 0;
  final Set<String> _disabled = {};
  final _sessions = StreamController<SessionState>.broadcast();
  SessionState _state = SessionState.idle;
  Timer? _timer;

  static const List<Tweak> _hogs = [
    {'kind': 'throttle_background', 'pid': 0, 'name': 'com.facebook.katana'},
    {'kind': 'throttle_background', 'pid': 0, 'name': 'com.dropbox.android'},
  ];

  static const _games = [
    MobileGame(package: 'com.activision.callofduty.shooter', label: 'Call of Duty: Mobile', enabled: true),
    MobileGame(package: 'com.miHoYo.GenshinImpact', label: 'Genshin Impact', enabled: true),
    MobileGame(package: 'com.supercell.brawlstars', label: 'Brawl Stars', enabled: true),
  ];

  // ---------------------------------------------------------------- OptimizerAgent

  @override
  Future<Capabilities> capabilities() async => const Capabilities(elevated: 'ready');

  @override
  Future<ScanResult> scan() async {
    await Future<void>.delayed(latency);
    if (_optimized) {
      return const ScanResult(
        score: 92,
        labelKey: 'health.excellent',
        headlineKey: 'headline.all_good',
        issues: [],
        oneClick: [],
        guided: [],
        tier: 'mid',
        processes: [ProcessRow(pid: 9001, name: 'com.activision.callofduty.shooter', cpuPct: 38.2, ramMb: 0)],
        ramUsedPct: 61,
      );
    }
    return ScanResult(
      score: 58,
      labelKey: 'health.needs_attention',
      headlineKey: 'headline.background_apps',
      issues: const [
        Issue(id: 'background_apps', severity: 7, titleKey: 'issue.background_apps', fix: _hogs, safeForOneClick: true),
        Issue(
          id: 'game_mode',
          severity: 4,
          titleKey: 'issue.game_mode',
          fix: [
            {'kind': 'enable_game_mode'},
          ],
          safeForOneClick: true,
        ),
        Issue(
          id: 'temp_files',
          severity: 2,
          titleKey: 'issue.temp_files',
          fix: [
            {'kind': 'clean_temp'},
          ],
          safeForOneClick: true,
        ),
        Issue(
          id: 'power_save',
          severity: 5,
          titleKey: 'issue.power_save',
          fix: [
            {'kind': 'open_system_screen', 'screen': 'battery_saver'},
          ],
          safeForOneClick: false,
        ),
      ],
      oneClick: [
        ..._hogs,
        {'kind': 'enable_game_mode'},
        {'kind': 'clean_temp'},
      ],
      guided: [
        {'kind': 'open_system_screen', 'screen': 'battery_saver'},
      ],
      tier: 'mid',
      processes: const [
        ProcessRow(pid: 4120, name: 'com.facebook.katana', cpuPct: 9.4, ramMb: 0),
        ProcessRow(pid: 8812, name: 'com.dropbox.android', cpuPct: 6.1, ramMb: 0),
        ProcessRow(pid: 2210, name: 'com.android.chrome', cpuPct: 2.3, ramMb: 0),
      ],
      ramUsedPct: 74,
    );
  }

  @override
  Future<ApplyResult> apply(List<Tweak> tweaks, {String label = 'one_click'}) async {
    await Future<void>.delayed(latency);
    _optimized = true;
    return ApplyResult(session: ++_session, applied: tweaks.where((t) => !t.isGuided).toList(), failed: 0);
  }

  @override
  Future<int> undo(int session) async {
    await Future<void>.delayed(latency ~/ 2);
    _optimized = false;
    return 3;
  }

  @override
  Future<bool> openScreen(String screen) async => true;

  @override
  Future<bool> requestElevation() async => true;

  @override
  Future<List<GameInfo>> games() async => _games
      .where((g) => !_disabled.contains(g.package))
      .map((g) => GameInfo(id: g.package, label: g.label))
      .toList();

  @override
  Future<void> gameStart(GameInfo game) async {
    _tick = 0;
    _timer?.cancel();
    _emit(_tickState(game.id, game.label));
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      _tick++;
      _emit(_tickState(game.id, game.label));
    });
  }

  @override
  Future<GameTick> gameTick() async {
    final s = _state;
    return GameTick(stats: s.stats, fpsCap: s.fpsCap, thermal: s.thermal, capTip: s.capTip);
  }

  @override
  Future<int> gameStop() async {
    _timer?.cancel();
    _timer = null;
    final label = _state.label ?? '';
    _emit(SessionState(active: false, summary: SessionSummary(label: label, minutes: 24, restored: 4)));
    return 4;
  }

  /// Primeros segundos con calor: el regulador baja a 45 FPS y la partida se estabiliza.
  SessionState _tickState(String pkg, String label) {
    final settling = _tick < 6;
    final avg = settling ? 54 + _rng.nextDouble() * 8 : 44.6 + _rng.nextDouble() * 0.4;
    final low = settling ? 33 + _rng.nextDouble() * 4 : 43 + _rng.nextDouble() * 1.5;
    return SessionState(
      active: true,
      package: pkg,
      label: label,
      auto: _auto,
      startedAt: DateTime.now().subtract(Duration(seconds: _tick)),
      applied: 4,
      stats: _tick < 2
          ? null
          : FrameStats(avgFps: avg, low1Fps: low, stuttersPerMin: settling ? 9 : 0.3, pacingCv: settling ? 0.22 : 0.04),
      fpsCap: _tick >= 3 ? 45 : 60,
      capReason: _tick >= 3 ? 'heat' : null,
      thermal: settling ? 'moderate' : 'light',
      batteryTempC: settling ? 41.5 : 38.2,
      measuresFps: true,
    );
  }

  void _emit(SessionState s) {
    _state = s;
    _sessions.add(s);
  }

  // ---------------------------------------------------------------- MobileExtras

  @override
  Future<BatteryStatus> batteryStatus() async => BatteryStatus(
        levelPct: 64,
        charging: false,
        tempC: 33.4,
        powerSave: true,
        appExempt: false,
        backgroundRestricted: false,
        standbyBucket: 'working_set',
        manufacturer: 'xiaomi',
        aggressiveOem: true,
        thermal: 'normal',
        thermalHeadroom: 0.42,
        elevated: 'ready',
        games: [
          for (final g in _games)
            GameBattery(package: g.package, label: g.label, exempt: g.package.startsWith('com.activision')),
        ],
      );

  @override
  Future<bool> autoGameMode() async => _auto;

  @override
  Future<void> setAutoGameMode(bool enabled) async {
    _auto = enabled;
  }

  @override
  Future<List<MobileGame>> mobileGames() async => [
        for (final g in _games) MobileGame(package: g.package, label: g.label, enabled: !_disabled.contains(g.package)),
      ];

  @override
  Future<void> setGameEnabled(String package, bool enabled) async {
    if (enabled) {
      _disabled.remove(package);
    } else {
      _disabled.add(package);
    }
  }

  @override
  Stream<SessionState> sessionStates() async* {
    yield _state;
    yield* _sessions.stream;
  }
}
