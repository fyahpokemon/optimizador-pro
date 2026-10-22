import 'dart:typed_data';

// Generados por `flutter_rust_bridge_codegen generate` (ver flutter_rust_bridge.yaml).
import '../bridge/api.dart' as rust;
import '../bridge/frb_generated.dart';
import 'core_api.dart';

/// Núcleo en Rust vía FFI. Llamar a [RustCore.init] una vez antes de usarlo.
class RustCore implements CoreApi {
  RustCore._();

  static Future<RustCore> init() async {
    await RustLib.init();
    return RustCore._();
  }

  @override
  Future<String> scan(String snapshotJson) => rust.scan(snapshotJson: snapshotJson, rulesJson: null);

  @override
  Future<String> profile(String snapshotJson) => rust.profile(snapshotJson: snapshotJson);

  @override
  Future<String> gameStart(String profileJson, String game) =>
      rust.gameStart(profileJson: profileJson, game: game, rulesJson: null);

  @override
  Future<void> gamePushFrames(List<double> frametimesMs) =>
      rust.gamePushFrames(frametimesMs: Float32List.fromList(frametimesMs));

  @override
  Future<String> gameTick(String snapshotJson) => rust.gameTick(snapshotJson: snapshotJson);

  @override
  Future<void> gameStop() => rust.gameStop();
}
