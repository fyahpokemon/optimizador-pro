/// Acceso al núcleo en Rust desde Dart (lo usa Android; en Windows el núcleo corre en el
/// servicio). La implementación real está en `core_ffi.dart`, que importa los bindings
/// generados por flutter_rust_bridge; así el resto de la app (y los tests con
/// [DemoAgent]) compila sin haber generado nada.
abstract interface class CoreApi {
  Future<String> scan(String snapshotJson);
  Future<String> profile(String snapshotJson);
  Future<String> gameStart(String profileJson, String game);
  Future<void> gamePushFrames(List<double> frametimesMs);
  Future<String> gameTick(String snapshotJson);
  Future<void> gameStop();
}
