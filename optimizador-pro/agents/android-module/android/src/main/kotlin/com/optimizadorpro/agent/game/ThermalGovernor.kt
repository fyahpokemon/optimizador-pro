package com.optimizadorpro.agent.game

/**
 * Regulador térmico de la partida.
 *
 * En un móvil el límite real de rendimiento sostenido es el calor: cuando el sistema
 * empieza a limitar la CPU/GPU, los FPS caen a saltos. Bajar un poco el tope de FPS
 * antes de llegar ahí mantiene la partida estable y fría. Con margen térmico de sobra
 * se vuelve a subir, escalón a escalón.
 *
 * Mismo criterio que el controlador de `core/profiles`: un cambio como mucho cada
 * [minChangeMs], y subir solo tras [coolBeforeRaiseMs] estable y frío.
 *
 * Estados térmicos: valores de `PowerManager.THERMAL_STATUS_*` (0 = normal … 6).
 */
class ThermalGovernor(
    /** Tope inicial del perfil (null = sin tope mientras no haya calor). */
    private val profileCap: Int?,
    /** Estado térmico a partir del cual se considera "calor" (1 = LIGHT en móviles). */
    private val hotStatus: Int = 1,
    private val minChangeMs: Long = 10_000,
    private val coolBeforeRaiseMs: Long = 60_000,
) {
    data class Decision(val cap: Int?, val reason: Reason)

    enum class Reason { HEAT, HEADROOM, STUTTER, RECOVERED }

    var cap: Int? = profileCap
        private set

    private var lastChangeMs: Long? = null
    private var coolSinceMs: Long? = null

    /** Un paso (cada segundo). Devuelve el nuevo tope si cambia; null si no hay que tocar nada. */
    fun step(nowMs: Long, thermalStatus: Int, headroom: Float?, stats: FrameStats?): Decision? {
        val nearLimit = headroom != null && headroom > 0.85f
        val hot = thermalStatus >= hotStatus || nearLimit
        val cool = thermalStatus < hotStatus && (headroom == null || headroom < 0.7f)
        val canChange = lastChangeMs?.let { nowMs - it >= minChangeMs } ?: true

        if (hot) {
            coolSinceMs = null
            if (!canChange) return null
            val lower = lowerCap(stats) ?: return null
            return change(nowMs, lower, if (nearLimit && thermalStatus < hotStatus) Reason.HEADROOM else Reason.HEAT)
        }

        // Sin calor pero con tirones y un tope ya puesto: bajar al nivel que el móvil sostiene.
        if (stats != null && !stats.stable && cap != null && canChange) {
            val sustainable = sustainableCap(stats.low1Fps)
            if (sustainable < cap!!) return change(nowMs, sustainable, Reason.STUTTER)
        }

        if (!cool || cap == profileCap) {
            if (!cool) coolSinceMs = null
            return null
        }
        val since = coolSinceMs ?: nowMs.also { coolSinceMs = it }
        if (nowMs - since < coolBeforeRaiseMs || !canChange) return null
        coolSinceMs = nowMs
        return change(nowMs, higherCap(), Reason.RECOVERED)
    }

    private fun change(nowMs: Long, newCap: Int?, reason: Reason): Decision? {
        if (newCap == cap) return null
        cap = newCap
        lastChangeMs = nowMs
        return Decision(newCap, reason)
    }

    /** Siguiente tope por debajo del actual, o el que sostiene el 1 % bajo si es menor. */
    private fun lowerCap(stats: FrameStats?): Int? {
        val current = cap ?: Int.MAX_VALUE
        val stepDown = CAPS.lastOrNull { it < current } ?: return null
        val sustainable = stats?.let { sustainableCap(it.low1Fps) } ?: stepDown
        return minOf(stepDown, sustainable)
    }

    /** Un escalón hacia arriba sin pasar del perfil; sin perfil, al llegar arriba se quita el tope. */
    private fun higherCap(): Int? {
        val current = cap ?: return null
        val next = CAPS.firstOrNull { it > current }
        return when {
            profileCap != null && (next == null || next > profileCap) -> profileCap
            next == null -> null
            else -> next
        }
    }

    companion object {
        val CAPS = listOf(30, 40, 45, 60, 90, 120)

        /** Mayor tope que el móvil sostiene: algo por debajo de su 1 % bajo actual. */
        fun sustainableCap(low1Fps: Double): Int = CAPS.lastOrNull { it <= low1Fps * 1.1 } ?: CAPS.first()

        /** Tope inicial por memoria y tasa de refresco (sin listas de SoC). */
        fun profileCapFor(ramGb: Double, refreshHz: Int?): Int? = when {
            ramGb <= 4.5 -> if ((refreshHz ?: 60) >= 90) 45 else 30
            ramGb <= 11.5 -> 60
            else -> null
        }
    }
}
