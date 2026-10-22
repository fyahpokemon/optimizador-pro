package com.optimizadorpro.agent.sensors

import com.optimizadorpro.agent.shizuku.Elevated

/**
 * CPU por proceso a partir de `dumpsys cpuinfo` (requiere Shizuku). Formato de las líneas:
 *   "  12% 1234/com.foo.bar: 10% user + 2% kernel"
 *   "34% TOTAL: 20% user + 12% kernel + ..."
 */
data class CpuInfo(val totalPct: Double, val processes: List<Proc>) {
    data class Proc(val pid: Int, val name: String, val cpuPct: Double, val isSystem: Boolean)

    companion object {
        private val LINE = Regex("""^\s*([\d.]+)%\s+(\d+)/([^:]+):""")
        private val TOTAL = Regex("""^\s*([\d.]+)%\s+TOTAL""")

        /** Procesos que nunca se tocan aunque gasten CPU. */
        private val SYSTEM_PREFIXES = listOf(
            "system_server", "surfaceflinger", "com.android.systemui", "android.hardware.",
            "/system/", "/vendor/", "kworker", "irq/", "logd", "audioserver", "media",
            "com.android.phone", "com.google.android.gms",
        )

        fun read(elevated: Elevated): CpuInfo? = runCatching { parse(elevated.exec("dumpsys cpuinfo")) }.getOrNull()

        fun parse(text: String): CpuInfo {
            var total = 0.0
            val procs = mutableListOf<Proc>()
            for (line in text.lineSequence()) {
                TOTAL.find(line)?.let { total = it.groupValues[1].toDouble() }
                    ?: LINE.find(line)?.let { m ->
                        val name = m.groupValues[3].trim()
                        procs += Proc(
                            pid = m.groupValues[2].toInt(),
                            name = name,
                            cpuPct = m.groupValues[1].toDouble(),
                            isSystem = SYSTEM_PREFIXES.any { name.startsWith(it) },
                        )
                    }
            }
            return CpuInfo(total.coerceAtMost(100.0), procs)
        }
    }
}
