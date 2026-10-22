package com.optimizadorpro.agent.game

import com.optimizadorpro.agent.shizuku.Elevated

/**
 * Frametimes de un juego leídos de SurfaceFlinger (requiere Shizuku).
 *
 * `dumpsys SurfaceFlinger --latency <capa>` devuelve el periodo de refresco en la primera
 * línea y después hasta 128 filas "deseado  real  listo" en nanosegundos. La columna central
 * (momento real de presentación) da el frametime como diferencia entre filas consecutivas.
 * Se llama cada segundo y solo se devuelven los frames nuevos desde la última lectura.
 */
class FrameSampler(private val elevated: Elevated, private val packageName: String) {
    private var layer: String? = null
    private var lastPresentNs = 0L

    fun available() = elevated.status() == Elevated.Status.READY

    private fun findLayer(): String? {
        val list = runCatching { elevated.exec("dumpsys SurfaceFlinger --list") }.getOrNull() ?: return null
        val candidates = list.lineSequence().map { it.trim() }.filter { packageName in it }.toList()
        // Preferir la SurfaceView (juegos con motor propio) y la capa BLAST activa.
        return candidates.firstOrNull { it.startsWith("SurfaceView") && "BLAST" in it }
            ?: candidates.firstOrNull { it.startsWith("SurfaceView") }
            ?: candidates.lastOrNull()
    }

    /** Frametimes nuevos en milisegundos. Vacío si no hay datos (sin Shizuku o capa no encontrada). */
    fun drain(): List<Float> {
        if (!available()) return emptyList()
        val name = layer ?: findLayer()?.also { layer = it } ?: return emptyList()
        val out = runCatching { elevated.exec("dumpsys SurfaceFlinger --latency '${name.replace("'", "")}'") }
            .getOrNull() ?: return emptyList()

        val presents = out.lineSequence().drop(1)
            .mapNotNull { line ->
                val cols = line.trim().split(Regex("\\s+"))
                cols.getOrNull(1)?.toLongOrNull()
            }
            // 0 = frame aún no presentado; Long.MAX_VALUE = pendiente.
            .filter { it in 1 until Long.MAX_VALUE }
            .toList()

        if (presents.isEmpty()) {
            layer = null // la capa pudo cambiar (p. ej. tras volver del menú): buscar de nuevo
            return emptyList()
        }
        val fresh = presents.filter { it > lastPresentNs }
        val base = if (lastPresentNs > 0) listOf(lastPresentNs) + fresh else fresh
        lastPresentNs = presents.last()
        return base.zipWithNext { a, b -> (b - a) / 1_000_000f }.filter { it in 0.5f..1000f }
    }
}
