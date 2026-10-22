import '../domain/models.dart';

/// Lo que la UI necesita de cada plataforma. Implementaciones:
/// - [WindowsAgent]: habla con el servicio por named pipe (el núcleo corre en el servicio).
/// - [AndroidAgent]: canal de métodos + núcleo en Rust vía flutter_rust_bridge.
/// - [DemoAgent]: datos simulados para desarrollo, capturas y tests de widgets.
abstract interface class OptimizerAgent {
  Future<Capabilities> capabilities();

  Future<ScanResult> scan();

  /// Aplica ajustes (los guiados se ignoran: van por [openScreen]).
  Future<ApplyResult> apply(List<Tweak> tweaks, {String label = 'one_click'});

  Future<int> undo(int session);

  Future<bool> openScreen(String screen);

  /// Android: pide permiso a Shizuku. Windows: siempre true.
  Future<bool> requestElevation();

  Future<List<GameInfo>> games();

  Future<void> gameStart(GameInfo game);

  /// Un paso del bucle de control (llamar cada segundo durante la partida).
  Future<GameTick> gameTick();

  /// Termina la partida y deshace sus cambios. Devuelve cuántos se restauraron.
  Future<int> gameStop();
}

class AgentUnavailable implements Exception {
  const AgentUnavailable(this.detail);
  final String detail;

  @override
  String toString() => 'AgentUnavailable: $detail';
}
