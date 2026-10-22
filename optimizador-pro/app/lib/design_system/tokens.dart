import 'package:flutter/material.dart';

/// Tokens del sistema visual (ver la guía de UI/UX del documento de arquitectura).
abstract final class Space {
  static const double xs = 4;
  static const double s = 8;
  static const double m = 16;
  static const double l = 24;
  static const double xl = 32;
  static const double xxl = 48;

  /// Margen lateral según el ancho: 16 en móvil, 24 en escritorio.
  static double gutter(BuildContext context) => MediaQuery.sizeOf(context).width < 600 ? m : l;
}

abstract final class Radii {
  static const double card = 16;
  static const double button = 20;
  static const double chip = 999;
}

abstract final class Motion {
  static const Duration fast = Duration(milliseconds: 150);
  static const Duration normal = Duration(milliseconds: 250);
  static const Curve curve = Curves.easeOutCubic;

  /// Respeta "reducir movimiento" del sistema.
  static Duration of(BuildContext context, Duration d) =>
      MediaQuery.of(context).disableAnimations ? Duration.zero : d;
}

/// Escala tipográfica: 32 / 22 / 17 / 15 / 13.
abstract final class TypeScale {
  static const double score = 32;
  static const double title = 22;
  static const double body = 17;
  static const double secondary = 15;
  static const double label = 13;
}

/// Un único acento + colores de estado. El color nunca es el único indicador:
/// cada estado lleva también icono y texto.
abstract final class Palette {
  static const Color accent = Color(0xFF2F6BFF);
  static const Color accentDark = Color(0xFF7FA2FF);

  static const Color good = Color(0xFF1E8E5A);
  static const Color goodDark = Color(0xFF5CCB94);
  static const Color warning = Color(0xFFB26A00);
  static const Color warningDark = Color(0xFFF2B04C);
  static const Color problem = Color(0xFFC62E3A);
  static const Color problemDark = Color(0xFFFF7A84);

  static const Color bgLight = Color(0xFFF6F7F9);
  static const Color surfaceLight = Color(0xFFFFFFFF);
  static const Color bgDark = Color(0xFF0F1115);
  static const Color surfaceDark = Color(0xFF181B21);
}

enum Health { good, warning, problem }

extension HealthColors on Health {
  Color color(Brightness b) => switch (this) {
        Health.good => b == Brightness.dark ? Palette.goodDark : Palette.good,
        Health.warning => b == Brightness.dark ? Palette.warningDark : Palette.warning,
        Health.problem => b == Brightness.dark ? Palette.problemDark : Palette.problem,
      };

  IconData get icon => switch (this) {
        Health.good => Icons.check_circle_rounded,
        Health.warning => Icons.error_rounded,
        Health.problem => Icons.warning_rounded,
      };

  static Health forScore(int score) => score >= 70
      ? Health.good
      : score >= 40
          ? Health.warning
          : Health.problem;
}
