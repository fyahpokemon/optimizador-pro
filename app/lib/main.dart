import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app.dart';
import 'platform/agent.dart';
import 'platform/android_agent.dart';
import 'platform/core_ffi.dart';
import 'platform/windows_agent.dart';
import 'state/providers.dart';

/// Punto de entrada de producción. Para la UI con datos simulados: `lib/main_demo.dart`.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final catalog = await _loadGameCatalog();

  final OptimizerAgent agent;
  if (Platform.isAndroid) {
    final android = AndroidAgent(await RustCore.init());
    final rules = await _loadProcessRules();
    // Sin await: no retrasa el primer fotograma.
    android
        .configure(catalog: catalog.android, whitelist: rules.whitelist, knownHeavy: rules.knownHeavy)
        .ignore();
    agent = android;
  } else if (Platform.isWindows) {
    agent = WindowsAgent(gameCatalog: catalog.windows);
  } else {
    throw UnsupportedError('Optimizador Pro solo funciona en Windows y Android');
  }

  runApp(ProviderScope(overrides: [agentProvider.overrideWithValue(agent)], child: const OptimizadorApp()));
}

Future<({Set<String> windows, Set<String> android})> _loadGameCatalog() async {
  final j = jsonDecode(await rootBundle.loadString('assets/game_catalog.json')) as Map<String, dynamic>;
  final windows = (j['windows_exe'] as List).cast<String>().map((s) => s.toLowerCase()).toSet();
  // Los paquetes Android distinguen mayúsculas (p. ej. com.miHoYo.GenshinImpact).
  final android = (j['android_packages'] as List).cast<String>().toSet();
  return (windows: windows, android: android);
}

/// Listas de `process_rules.json`: en Android solo interesan los nombres de paquete.
Future<({Set<String> whitelist, Set<String> knownHeavy})> _loadProcessRules() async {
  final j = jsonDecode(await rootBundle.loadString('assets/process_rules.json')) as Map<String, dynamic>;
  Set<String> packages(String k) => (j[k] as List).cast<String>().where((s) => !s.endsWith('.exe')).toSet();
  return (whitelist: packages('whitelist'), knownHeavy: packages('known_heavy'));
}
