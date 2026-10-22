import 'dart:math';

import 'package:flutter/material.dart';

import '../../l10n/strings.dart';
import '../tokens.dart';

/// Anillo de salud 0-100 con número y etiqueta. Con texto grande (≥ 1.6x) se convierte en
/// una fila compacta para no cortar contenido.
class HealthRing extends StatelessWidget {
  const HealthRing({super.key, required this.score, required this.labelKey, this.busy = false});

  final int? score;
  final String labelKey;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final brightness = Theme.of(context).brightness;
    final health = HealthColors.forScore(score ?? 100);
    final color = score == null ? Theme.of(context).colorScheme.outline : health.color(brightness);
    final label = context.tr(labelKey);
    final scale = MediaQuery.textScalerOf(context).scale(1);

    final semantics = score == null
        ? context.tr('home.scan_first')
        : context.tr('health.semantics', {'score': '$score', 'label': label});

    final number = Text(
      score?.toString() ?? '—',
      style: Theme.of(context).textTheme.displaySmall?.copyWith(fontSize: 48),
    );
    final caption = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (score != null) Icon(health.icon, color: color, size: 20),
        const SizedBox(width: Space.xs),
        Flexible(child: Text(label, style: Theme.of(context).textTheme.bodyLarge)),
      ],
    );

    if (scale >= 1.6) {
      return Semantics(
        label: semantics,
        excludeSemantics: true,
        child: Row(children: [number, const SizedBox(width: Space.m), Expanded(child: caption)]),
      );
    }

    return Semantics(
      label: semantics,
      liveRegion: true,
      excludeSemantics: true,
      child: SizedBox.square(
        dimension: 200,
        child: TweenAnimationBuilder<double>(
          tween: Tween(end: (score ?? 0) / 100),
          duration: Motion.of(context, const Duration(milliseconds: 700)),
          curve: Motion.curve,
          builder: (context, v, _) => CustomPaint(
            painter: _RingPainter(
              progress: v,
              color: color,
              track: Theme.of(context).colorScheme.surfaceContainerHighest,
              spinning: busy,
            ),
            child: Center(child: Column(mainAxisSize: MainAxisSize.min, children: [number, caption])),
          ),
        ),
      ),
    );
  }
}

class _RingPainter extends CustomPainter {
  _RingPainter({required this.progress, required this.color, required this.track, required this.spinning});

  final double progress;
  final Color color;
  final Color track;
  final bool spinning;

  @override
  void paint(Canvas canvas, Size size) {
    const stroke = 14.0;
    final rect = Offset.zero & size;
    final arcRect = rect.deflate(stroke / 2);
    final base = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(arcRect, 0, 2 * pi, false, base..color = track);
    if (!spinning) {
      canvas.drawArc(arcRect, -pi / 2, 2 * pi * progress, false, base..color = color);
    }
  }

  @override
  bool shouldRepaint(_RingPainter old) =>
      old.progress != progress || old.color != color || old.spinning != spinning || old.track != track;
}
