package com.optimizadorpro.agent.game

import org.json.JSONObject
import kotlin.math.ceil
import kotlin.math.sqrt

/**
 * Análisis de frametimes. Mismo algoritmo y umbrales que `core/stutter` en Rust: en Android
 * el bucle de la partida corre aquí porque la UI de Flutter está en segundo plano.
 */
data class FrameStats(
    val frames: Int,
    val avgFps: Double,
    val low1Fps: Double,
    val stutters: Int,
    val stuttersPerMin: Double,
    val pacingCv: Double,
) {
    val stable: Boolean get() = low1Fps >= avgFps * 0.75 && stuttersPerMin < 2.0 && pacingCv < 0.15

    fun toJson(): JSONObject = JSONObject()
        .put("avg_fps", avgFps)
        .put("low1_fps", low1Fps)
        .put("stutters_per_min", stuttersPerMin)
        .put("pacing_cv", pacingCv)
        .put("stable", stable)

    companion object {
        const val MIN_FRAMES = 30
        private const val WINDOW = 15
        private const val STUTTER_RATIO = 2.0
        private const val STUTTER_MIN_EXTRA_MS = 8.0

        fun analyze(frametimesMs: List<Double>): FrameStats? {
            val ft = frametimesMs.filter { it.isFinite() && it > 0.0 }
            if (ft.size < MIN_FRAMES) return null
            val n = ft.size.toDouble()
            val total = ft.sum()
            val mean = total / n
            val sorted = ft.sorted()
            val p99 = sorted[(ceil(n * 0.99).toInt().coerceIn(1, ft.size)) - 1]
            val variance = ft.sumOf { (it - mean) * (it - mean) } / n

            var stutters = 0
            for (i in WINDOW until ft.size) {
                val med = ft.subList(i - WINDOW, i).sorted()[WINDOW / 2]
                if (ft[i] > med * STUTTER_RATIO && ft[i] - med > STUTTER_MIN_EXTRA_MS) stutters++
            }
            return FrameStats(
                frames = ft.size,
                avgFps = 1000.0 / mean,
                low1Fps = 1000.0 / p99,
                stutters = stutters,
                stuttersPerMin = stutters / (total / 60_000.0),
                pacingCv = sqrt(variance) / mean,
            )
        }
    }
}

/** Ventana deslizante de los últimos [seconds] segundos de frametimes. */
class FrameWindow(seconds: Double = 10.0) {
    private val maxMs = seconds * 1000.0
    private val data = ArrayDeque<Double>()
    private var sumMs = 0.0

    fun push(ms: Double) {
        if (!ms.isFinite() || ms <= 0.0) return
        data.addLast(ms)
        sumMs += ms
        while (sumMs > maxMs && data.isNotEmpty()) sumMs -= data.removeFirst()
    }

    fun clear() {
        data.clear()
        sumMs = 0.0
    }

    fun stats(): FrameStats? = FrameStats.analyze(data.toList())
}
