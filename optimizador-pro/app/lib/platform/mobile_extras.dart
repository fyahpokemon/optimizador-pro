import '../domain/models.dart';

/// Funciones exclusivas del móvil: batería, modo juego automático y partida en vivo.
/// La implementan [AndroidAgent] y [DemoAgent]; la UI muestra las pestañas de Juegos y
/// Batería solo si el agente las ofrece.
abstract interface class MobileExtras {
  Future<BatteryStatus> batteryStatus();

  Future<bool> autoGameMode();

  /// Activa o desactiva la detección automática (arranca o para el servicio en primer plano).
  Future<void> setAutoGameMode(bool enabled);

  Future<List<MobileGame>> mobileGames();

  Future<void> setGameEnabled(String package, bool enabled);

  /// Estado de la partida: se emite al suscribirse y cada segundo mientras se juega.
  Stream<SessionState> sessionStates();
}

class MobileGame {
  const MobileGame({required this.package, required this.label, required this.enabled});

  factory MobileGame.fromJson(Map<String, dynamic> j) => MobileGame(
        package: j['package'] as String,
        label: j['label'] as String,
        enabled: j['enabled'] as bool? ?? true,
      );

  final String package;
  final String label;
  final bool enabled;
}

class GameBattery {
  const GameBattery({required this.package, required this.label, required this.exempt});

  final String package;
  final String label;
  final bool exempt;
}

class BatteryStatus {
  const BatteryStatus({
    required this.levelPct,
    required this.charging,
    required this.tempC,
    required this.powerSave,
    required this.appExempt,
    required this.backgroundRestricted,
    required this.standbyBucket,
    required this.manufacturer,
    required this.aggressiveOem,
    required this.thermal,
    required this.thermalHeadroom,
    required this.elevated,
    required this.games,
  });

  factory BatteryStatus.fromJson(Map<String, dynamic> j) => BatteryStatus(
        levelPct: (j['level_pct'] as num?)?.toInt(),
        charging: j['charging'] as bool,
        tempC: (j['temp_c'] as num).toDouble(),
        powerSave: j['power_save'] as bool,
        appExempt: j['app_exempt'] as bool,
        backgroundRestricted: j['background_restricted'] as bool,
        standbyBucket: j['standby_bucket'] as String,
        manufacturer: j['manufacturer'] as String,
        aggressiveOem: j['aggressive_oem'] as bool,
        thermal: j['thermal'] as String,
        thermalHeadroom: (j['thermal_headroom'] as num?)?.toDouble(),
        elevated: j['elevated'] as String,
        games: (j['games'] as List)
            .cast<Map<String, dynamic>>()
            .map((g) => GameBattery(
                  package: g['package'] as String,
                  label: g['label'] as String,
                  exempt: g['battery_exempt'] as bool,
                ))
            .toList(),
      );

  final int? levelPct;
  final bool charging;
  final double tempC;
  final bool powerSave;

  /// Optimizador Pro exento de la optimización de batería (necesario para el modo automático).
  final bool appExempt;

  /// "Restringida" en Ajustes > Batería: el sistema no deja trabajar en segundo plano.
  final bool backgroundRestricted;

  /// active | working_set | frequent | rare | restricted
  final String standbyBucket;
  final String manufacturer;
  final bool aggressiveOem;
  final String thermal;
  final double? thermalHeadroom;
  final String elevated;
  final List<GameBattery> games;

  bool get canRunInBackground => appExempt && !backgroundRestricted;
  bool get bucketLimited => standbyBucket == 'rare' || standbyBucket == 'restricted';

  /// Pasos pendientes para que el modo juego funcione sin cortes.
  int get pendingSteps =>
      (canRunInBackground ? 0 : 1) + (powerSave ? 1 : 0) + (bucketLimited ? 1 : 0) + (aggressiveOem ? 1 : 0);
}

class SessionSummary {
  const SessionSummary({required this.label, required this.minutes, required this.restored});

  final String label;
  final int minutes;
  final int restored;
}

class SessionState {
  const SessionState({
    required this.active,
    this.package,
    this.label,
    this.auto = false,
    this.startedAt,
    this.applied = 0,
    this.stats,
    this.fpsCap,
    this.capTip,
    this.capReason,
    this.thermal = 'normal',
    this.batteryTempC,
    this.measuresFps = false,
    this.summary,
  });

  factory SessionState.fromJson(Map<String, dynamic> j) {
    final s = j['summary'];
    return SessionState(
      active: j['active'] as bool,
      package: j['package'] as String?,
      label: j['label'] as String?,
      auto: j['auto'] as bool? ?? false,
      startedAt: j['started_ms'] == null
          ? null
          : DateTime.fromMillisecondsSinceEpoch((j['started_ms'] as num).toInt()),
      applied: (j['applied'] as num?)?.toInt() ?? 0,
      stats: j['stats'] is Map<String, dynamic> ? FrameStats.fromJson(j['stats'] as Map<String, dynamic>) : null,
      fpsCap: (j['fps_cap'] as num?)?.toInt(),
      capTip: (j['cap_tip'] as num?)?.toInt(),
      capReason: j['cap_reason'] as String?,
      thermal: j['thermal'] as String? ?? 'normal',
      batteryTempC: (j['battery_temp_c'] as num?)?.toDouble(),
      measuresFps: j['measures_fps'] as bool? ?? false,
      summary: s is Map<String, dynamic>
          ? SessionSummary(
              label: s['label'] as String,
              minutes: (s['minutes'] as num).toInt(),
              restored: (s['restored'] as num).toInt(),
            )
          : null,
    );
  }

  static const idle = SessionState(active: false);

  final bool active;
  final String? package;
  final String? label;
  final bool auto;
  final DateTime? startedAt;
  final int applied;
  final FrameStats? stats;
  final int? fpsCap;
  final int? capTip;

  /// heat | headroom | stutter | recovered: por qué cambió el límite por última vez.
  final String? capReason;
  final String thermal;
  final double? batteryTempC;
  final bool measuresFps;
  final SessionSummary? summary;
}
