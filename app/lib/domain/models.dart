/// Modelos de la UI. Reflejan el JSON del núcleo en Rust (`model` y `optimizador_core`);
/// los ajustes (`TweakKind`) se mantienen como mapas para no duplicar el contrato.
library;

typedef Tweak = Map<String, dynamic>;

extension TweakX on Tweak {
  String get kind => this['kind'] as String;
  bool get isGuided => kind == 'open_system_screen';
}

class Issue {
  const Issue({
    required this.id,
    required this.severity,
    required this.titleKey,
    required this.fix,
    required this.safeForOneClick,
  });

  factory Issue.fromJson(Map<String, dynamic> j) => Issue(
        id: j['id'] as String,
        severity: (j['severity'] as num).toInt(),
        titleKey: j['title_key'] as String,
        fix: (j['fix'] as List).cast<Map<String, dynamic>>(),
        safeForOneClick: j['safe_for_one_click'] as bool,
      );

  final String id;
  final int severity;
  final String titleKey;
  final List<Tweak> fix;
  final bool safeForOneClick;
}

class ProcessRow {
  const ProcessRow({required this.pid, required this.name, required this.cpuPct, required this.ramMb});

  factory ProcessRow.fromJson(Map<String, dynamic> j) => ProcessRow(
        pid: (j['pid'] as num).toInt(),
        name: j['name'] as String,
        cpuPct: (j['cpu_pct'] as num).toDouble(),
        ramMb: (j['ram_mb'] as num).toInt(),
      );

  final int pid;
  final String name;
  final double cpuPct;
  final int ramMb;
}

class ScanResult {
  const ScanResult({
    required this.score,
    required this.labelKey,
    required this.headlineKey,
    required this.issues,
    required this.oneClick,
    required this.guided,
    required this.tier,
    this.processes = const [],
    this.thermal = 'normal',
    this.ramUsedPct,
  });

  /// `scan` = salida de `optimizador_core::scan`; `snapshot` = instantánea que la originó.
  factory ScanResult.fromJson(Map<String, dynamic> scan, [Map<String, dynamic>? snapshot]) => ScanResult(
        score: (scan['score'] as num).toInt(),
        labelKey: scan['label_key'] as String,
        headlineKey: scan['headline_key'] as String,
        issues: (scan['issues'] as List).map((e) => Issue.fromJson(e as Map<String, dynamic>)).toList(),
        oneClick: (scan['one_click'] as List).cast<Map<String, dynamic>>(),
        guided: (scan['guided'] as List).cast<Map<String, dynamic>>(),
        tier: scan['tier'] as String,
        processes: ((snapshot?['processes'] as List?) ?? const [])
            .map((e) => ProcessRow.fromJson(e as Map<String, dynamic>))
            .where((p) => p.cpuPct > 0.1)
            .toList()
          ..sort((a, b) => b.cpuPct.compareTo(a.cpuPct)),
        thermal: (snapshot?['thermal'] as String?) ?? 'normal',
        ramUsedPct: (snapshot?['ram_used_pct'] as num?)?.toDouble(),
      );

  final int score;
  final String labelKey;
  final String headlineKey;
  final List<Issue> issues;
  final List<Tweak> oneClick;
  final List<Tweak> guided;
  final String tier;
  final List<ProcessRow> processes;
  final String thermal;
  final double? ramUsedPct;
}

class ApplyResult {
  const ApplyResult({required this.session, required this.applied, required this.failed});

  final int session;
  final List<Tweak> applied;
  final int failed;
}

class Capabilities {
  const Capabilities({this.elevated = 'unsupported', this.usageAccess = true, this.measuresFps = true});

  /// Android: unsupported | not_running | needs_permission | ready. Windows: siempre "ready".
  final String elevated;
  final bool usageAccess;
  final bool measuresFps;
}

class GameInfo {
  const GameInfo({required this.id, required this.label, this.pid});

  /// Paquete (Android) o nombre del ejecutable (Windows).
  final String id;
  final String label;
  final int? pid;
}

class FrameStats {
  const FrameStats({required this.avgFps, required this.low1Fps, required this.stuttersPerMin, required this.pacingCv});

  factory FrameStats.fromJson(Map<String, dynamic> j) => FrameStats(
        avgFps: (j['avg_fps'] as num).toDouble(),
        low1Fps: (j['low1_fps'] as num).toDouble(),
        stuttersPerMin: (j['stutters_per_min'] as num).toDouble(),
        pacingCv: (j['pacing_cv'] as num).toDouble(),
      );

  final double avgFps;
  final double low1Fps;
  final double stuttersPerMin;
  final double pacingCv;

  /// Mismo criterio que `stutter::is_stable` en el núcleo.
  bool get stable => low1Fps >= avgFps * 0.75 && stuttersPerMin < 2 && pacingCv < 0.15;
}

class GameTick {
  const GameTick({this.stats, this.fpsCap, this.thermal = 'normal', this.capTip});

  final FrameStats? stats;
  final int? fpsCap;
  final String thermal;

  /// Tope recomendado que el usuario debe fijar en el juego (cuando el agente no puede).
  final int? capTip;
}

class HistoryEntry {
  const HistoryEntry({required this.session, required this.when, required this.applied, this.undone = false});

  final int session;
  final DateTime when;
  final List<Tweak> applied;
  final bool undone;

  HistoryEntry copyWith({bool? undone}) =>
      HistoryEntry(session: session, when: when, applied: applied, undone: undone ?? this.undone);
}
