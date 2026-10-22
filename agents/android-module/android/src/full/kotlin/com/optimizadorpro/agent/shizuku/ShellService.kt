package com.optimizadorpro.agent.shizuku

import java.util.concurrent.TimeUnit
import kotlin.system.exitProcess

/**
 * Corre en un proceso lanzado por Shizuku con uid "shell". Solo acepta los comandos de la
 * lista permitida: aunque otro componente lograra enlazarse, no podría ejecutar cualquier cosa.
 */
class ShellService : IShellService.Stub() {

    private val allowedPrefixes = listOf(
        "cmd game ",
        "cmd appops set ",
        "cmd appops get ",
        "cmd deviceidle whitelist",
        "pm trim-caches ",
        "dumpsys SurfaceFlinger --latency",
        "dumpsys SurfaceFlinger --list",
        "dumpsys cpuinfo",
        "am force-stop ",
    )

    override fun exec(command: String): String {
        if (allowedPrefixes.none { command.startsWith(it) } || command.any { it in ";&|`$><\n" }) {
            return "ERR: comando no permitido"
        }
        return try {
            val p = ProcessBuilder("sh", "-c", command).redirectErrorStream(false).start()
            val out = p.inputStream.bufferedReader().readText()
            val err = p.errorStream.bufferedReader().readText()
            if (!p.waitFor(10, TimeUnit.SECONDS)) {
                p.destroy()
                "ERR: tiempo agotado"
            } else if (p.exitValue() != 0) {
                "ERR: ${err.ifBlank { out }}"
            } else out
        } catch (e: Exception) {
            "ERR: ${e.message}"
        }
    }

    override fun destroy() {
        exitProcess(0)
    }
}
