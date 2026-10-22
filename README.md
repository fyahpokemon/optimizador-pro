# Optimizador Pro (Windows y Android)

Rendimiento estable para jugar: diagnóstico en lenguaje sencillo, "Optimizar con 1 clic"
reversible y un modo juego que corrige en vivo solo la causa real de los tirones.

Arquitectura y decisiones: ver el documento de arquitectura (Claude Docs) que acompaña al
proyecto. Resumen:

```
app/                      Flutter (UI): inicio, resultados, modo juego, avanzado, historial
core/                     Rust compartido (sin acceso al sistema)
  model/                  Snapshot, Issue, TweakKind, Profile (contrato JSON)
  rules/                  motor de reglas: Snapshot → problemas + arreglos
  scoring/                puntuación de salud 0-100
  stutter/                análisis de frametimes (1 % bajo, tirones/min, regularidad)
  profiles/               perfiles por hardware + controlador del modo juego
  journal/                registro de cambios y reversión (SQLite)
  api/                    funciones expuestas a Flutter (flutter_rust_bridge) y a los agentes
agents/
  windows-service/        servicio Rust con privilegios: sensores, ajustes, PresentMon, named pipe
  android-module/         plugin Flutter en Kotlin: sensores, ajustes, Shizuku (sabor "full")
data/                     listas de procesos, catálogo de juegos, pantallas OEM
tools/                    instalación del servicio en desarrollo, prueba del pipe
.github/workflows/ci.yml  tests de núcleo, build del servicio, tests de UI y del agente Android
```

## Estado de esta entrega

| Parte | Cómo se ha verificado |
| --- | --- |
| Núcleo Rust (7 crates) | Compilado con las versiones reales de serde, serde_json, anyhow y rusqlite (SQLite incluido): 42 tests en verde, clippy sin avisos, rustfmt |
| Servicio de Windows | Compilado contra el código real de `windows` 0.58. `sysinfo`, `wmi`, `windows-service` y `walkdir` se comprobaron contra firmas copiadas de su código fuente (no se pudo compilar para el target de Windows) |
| Agente Android (Kotlin) | Sabores `play` y `full` compilados con Kotlin 2.0.21 contra `android.jar` API 35; APIs de Flutter y Shizuku comprobadas contra su código fuente; tests de `CpuInfo` en verde |
| App Flutter | Revisada a mano y APIs comprobadas contra el código de Dart, Flutter, Riverpod 2.6.1 y win32_registry 1.1.5. No se pudo compilar (sin SDK de Flutter) |

La CI (`.github/workflows/ci.yml`) es la prueba final: compila y prueba todo en Windows,
Linux y Android.

## Instalar la APK en el móvil (sin ordenador)

GitHub compila la APK automáticamente (`.github/workflows/android-apk.yml`):

1. Sube el proyecto a un repositorio de GitHub. Con cada push a `main` se compila la APK
   (unos 15–20 min la primera vez; luego menos, gracias a la caché).
2. En el móvil abre `https://github.com/<usuario>/<repo>/releases/download/ultima/Optimizador-Pro.apk`
   (enlace fijo: siempre es la última compilación de `main`). Con el repositorio privado,
   inicia sesión en GitHub en el navegador antes.
3. Abre el archivo descargado y permite «Instalar apps desconocidas» para el navegador si
   Android lo pide.
4. Para versiones estables, crea una etiqueta `v0.2.0` (por ejemplo desde la web: Releases ›
   Draft a new release › Choose a tag): se publica una Release con su APK.

**Firma (recomendado, una sola vez):** sin clave propia la APK se firma con una clave de
depuración distinta en cada compilación, y Android no deja instalar una encima de otra.
Ejecuta Actions › «Crear clave de firma» › Run workflow, descarga el artefacto
`clave-de-firma` y crea los secretos que indica su `LEEME.txt` (Settings › Secrets and
variables › Actions). Guarda la copia de seguridad de la clave. Desde ese momento, cada APK
se instala encima de la anterior.

Ejecución manual: Actions › «APK de Android» › Run workflow permite elegir el sabor `full`
(con Shizuku, por defecto) o `play` (solo APIs públicas).

## Puesta en marcha (desarrollo)

Requisitos: Rust estable, Flutter 3.47.7 (la que usa la CI), JDK 17 y Android SDK (para
Android), Visual Studio Build Tools con C++ (para Windows).

1. **Núcleo**
   ```bash
   cargo test --workspace --exclude optimizador-service
   ```
2. **Proyecto Android del host**: los archivos de Gradle y el manifiesto están en
   `app/android` (AGP 9.1, Kotlin 2.4, minSdk 29, sabor del agente con la variable
   `AGENT_FLAVOR`, `full` por defecto). `flutter create` completa el resto sin tocarlos:
   ```bash
   cd app
   flutter create --platforms=windows,android --org com.optimizadorpro --project-name optimizador_pro .
   ```
3. **Bindings y biblioteca nativa del núcleo** (Android usa el núcleo vía FFI; Windows lo usa
   dentro del servicio):
   ```bash
   cargo install flutter_rust_bridge_codegen --version 2.11.1 --locked
   cargo install cargo-ndk
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   (cd app && flutter pub get && flutter_rust_bridge_codegen generate)   # cada vez que cambie core/api/src/api.rs
   cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 --platform 29 \
     -o app/android/app/src/main/jniLibs build --release -p optimizador_core --features frb
   ```
4. **UI sin dispositivo** (datos simulados, ideal para diseño y capturas):
   ```bash
   flutter run -t lib/main_demo.dart
   flutter test
   ```
5. **Windows real**
   ```powershell
   cargo build -p optimizador-service --release
   # Copia PresentMon.exe (https://github.com/GameTechDev/PresentMon, MIT) a target\release\bin\
   .\tools\dev-install-service.ps1          # como administrador
   .\tools\pipe-ping.ps1                    # comprobación rápida
   cd app; flutter run -d windows
   ```
   Depuración sin instalar el servicio: `optimizador-service.exe --console` en una consola
   de administrador.
6. **Android real**
   ```bash
   cd app && flutter run -d <dispositivo>
   ```
   Concede "Acceso de uso" cuando la app lo pida. Para medir FPS y frenar apps en segundo
   plano, usa el sabor `full` e instala y abre [Shizuku](https://shizuku.rikka.app/).

## Android: cómo funciona en el móvil

- **Pestañas**: Inicio · Juegos · Batería · Más (`app/lib/features/shell/mobile_shell.dart`).
- **Batería** (`features/battery/battery_screen.dart` + `battery/BatteryInspector.kt`): comprueba si la app
  puede trabajar en segundo plano, el ahorro de batería, el ajuste del fabricante (Xiaomi, Huawei,
  Samsung…), el grupo de uso de Android y qué juegos están exentos. Cada fila lleva a la pantalla exacta
  del sistema y se vuelve a leer al volver.
- **Optimización en primer plano**: durante la partida la UI de Flutter está en segundo plano, así que
  todo corre en Kotlin dentro de un servicio en primer plano (`game/GameModeService.kt`):
  - `ForegroundWatcher` detecta el juego (solo con la pantalla encendida, cada 2 s).
  - `GameSessionManager` aplica el modo rendimiento, exime el juego de Doze y frena apps lastre
    (con Shizuku), todo en el journal; al salir del juego lo deshace.
  - `ThermalGovernor` baja el límite de FPS antes de que el móvil se frene por calor y lo vuelve a
    subir cuando se enfría (mismo criterio que el núcleo en Rust).
  - La notificación muestra el estado y un botón «Terminar».
- **Sin Shizuku** (versión de Google Play) se vigilan temperatura y estabilidad y el límite de FPS se
  recomienda en lugar de aplicarse.
- Tests: `GameLogicTest.kt` (estadísticas de frames y regulador térmico) y `home_flow_test.dart`
  (pestañas Batería y Juegos con el agente de demostración).

## Principios que el código respeta

- **Medir antes de tocar.** Cada ajuste nace de una regla con causa medida.
- **Todo reversible.** Se guarda el valor previo antes de aplicar; las partidas
  interrumpidas se deshacen solas al volver a arrancar.
- **Frenar, no matar.** Los procesos lastre bajan de prioridad (EcoQoS en Windows 11,
  restricción de segundo plano en Android); nunca se cierran en bucle.
- **Nada que toque el juego.** Sin inyección, sin overclock, sin tocar archivos del juego:
  seguro con anti-cheat. El tope de FPS se fija por el sistema o se recomienda.
- **Honestidad en la UI.** Antes/después con datos medidos; si el equipo ya está bien,
  se dice y no se cambia nada.

## Contrato JSON

El mismo JSON viaja entre núcleo, servicio, plugin y UI:

- `Snapshot` / `TweakKind` / `Issue`: `core/model/src/lib.rs` (serde, `snake_case`,
  `TweakKind` etiquetado con `"kind"`).
- Peticiones del pipe de Windows: `agents/windows-service/src/protocol.rs`.
- Canal Android `optimizador/android`: `agents/android-module/android/src/main/kotlin/.../OptimizadorPlugin.kt`.

## Textos

`app/lib/l10n/strings.dart` (español e inglés). Cada regla nueva del núcleo necesita sus
claves `issue.<id>` y `headline.<id>`.

## Pendiente para la versión 1.0 (según el plan del documento)

- Empaquetado MSIX con el servicio (`desktop6:Service`) y firma EV.
- Integración NVAPI / AMD ADLX para fijar topes de FPS por driver (Avanzado).
- Gráfico en vivo de frametime y temperatura en Avanzado.
- Persistir el historial de la UI (hoy vive en memoria; el journal del agente sí es persistente).
- Fuente Figtree (la del diseño, licencia OFL) en `app/assets/fonts` y declarada en `pubspec.yaml`; mientras no esté, la app usa la del sistema.
- Verificar las pantallas OEM de `data/oem_screens.json` en dispositivos reales.
