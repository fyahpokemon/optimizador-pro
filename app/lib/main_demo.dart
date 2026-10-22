import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app.dart';
import 'platform/demo_agent.dart';
import 'state/providers.dart';

/// UI con datos simulados (sin servicio, sin núcleo nativo):
///   flutter run -t lib/main_demo.dart -d windows   (o chrome, macos, linux)
void main() {
  runApp(
    ProviderScope(
      overrides: [agentProvider.overrideWithValue(DemoAgent())],
      child: const OptimizadorApp(),
    ),
  );
}
