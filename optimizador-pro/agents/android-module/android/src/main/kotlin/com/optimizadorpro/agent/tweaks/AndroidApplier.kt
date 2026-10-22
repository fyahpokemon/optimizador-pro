package com.optimizadorpro.agent.tweaks

import android.content.Context
import com.optimizadorpro.agent.oem.OemIntents
import com.optimizadorpro.agent.shizuku.Elevated
import com.optimizadorpro.agent.shizuku.requireSafePackage
import org.json.JSONObject

/**
 * Aplica los `TweakKind` del núcleo (mismo JSON: {"kind": "...", ...}) y sabe leer el
 * valor previo y restaurarlo. Lo que necesita elevación falla con un mensaje claro si
 * Shizuku no está listo; el journal lo registra como "no aplicado".
 */
class AndroidApplier(private val ctx: Context, private val elevated: Elevated) {

    private val ready get() = elevated.status() == Elevated.Status.READY

    /** Juego activo, para los ajustes que se aplican "a este juego". */
    var activeGame: String? = null

    fun readCurrent(tweak: JSONObject): JSONObject = when (tweak.getString("kind")) {
        "throttle_background" -> {
            val pkg = requireSafePackage(tweak.getString("name"))
            val mode = runCatching { elevated.exec("cmd appops get $pkg RUN_ANY_IN_BACKGROUND") }.getOrDefault("")
            JSONObject().put("mode", if ("ignore" in mode) "ignore" else "allow")
        }
        "exempt_from_battery_opt" -> {
            val pkg = requireSafePackage(tweak.getString("package"))
            val list = runCatching { elevated.exec("cmd deviceidle whitelist") }.getOrDefault("")
            JSONObject().put("whitelisted", list.lineSequence().any { it.trim().endsWith(",$pkg,") || it.contains(",$pkg,") })
        }
        "enable_game_mode", "set_fps_cap" -> {
            val pkg = gamePackage(tweak)
            val mode = runCatching { elevated.exec("cmd game mode get $pkg") }.getOrDefault("")
            JSONObject().put("mode", parseGameMode(mode))
        }
        else -> JSONObject()
    }

    fun apply(tweak: JSONObject) {
        when (tweak.getString("kind")) {
            "throttle_background" -> {
                needElevated()
                val pkg = requireSafePackage(tweak.getString("name"))
                if (pkg == activeGame) return // nunca frenar el juego
                elevated.exec("cmd appops set $pkg RUN_ANY_IN_BACKGROUND ignore")
            }
            "exempt_from_battery_opt" -> {
                needElevated()
                elevated.exec("cmd deviceidle whitelist +${requireSafePackage(tweak.getString("package"))}")
            }
            "enable_game_mode" -> {
                needElevated()
                // Android 12+: modo "performance" del sistema para ese juego (si el juego lo admite).
                elevated.exec("cmd game mode performance ${gamePackage(tweak)}")
            }
            "set_fps_cap" -> {
                needElevated()
                val fps = tweak.getInt("fps")
                val pkg = gamePackage(tweak)
                if (fps == 0) {
                    // Sin tope: quitar la intervención y dejar el modo rendimiento.
                    elevated.exec("cmd game reset $pkg")
                    elevated.exec("cmd game mode performance $pkg")
                } else {
                    require(fps in 15..240) { "FPS fuera de rango" }
                    // Intervención de modo de juego (Android 13+, según el dispositivo).
                    elevated.exec("cmd game set --mode 2 --fps $fps $pkg")
                }
            }
            "clean_temp" -> {
                ctx.cacheDir.listFiles()?.forEach { it.deleteRecursively() }
                if (ready) runCatching { elevated.exec("pm trim-caches 999G") }
            }
            "open_system_screen" -> {
                if (!OemIntents.open(ctx, tweak.getString("screen"))) error("Pantalla no disponible")
            }
            else -> error("Ajuste no disponible en Android: ${tweak.getString("kind")}")
        }
    }

    fun restore(tweak: JSONObject, previous: JSONObject) {
        when (tweak.getString("kind")) {
            "throttle_background" -> {
                val pkg = requireSafePackage(tweak.getString("name"))
                elevated.exec("cmd appops set $pkg RUN_ANY_IN_BACKGROUND ${previous.optString("mode", "allow")}")
            }
            "exempt_from_battery_opt" -> if (!previous.optBoolean("whitelisted", false)) {
                elevated.exec("cmd deviceidle whitelist -${requireSafePackage(tweak.getString("package"))}")
            }
            "enable_game_mode", "set_fps_cap" -> {
                val pkg = gamePackage(tweak)
                if (tweak.getString("kind") == "set_fps_cap") runCatching { elevated.exec("cmd game reset $pkg") }
                elevated.exec("cmd game mode ${previous.optString("mode", "standard")} $pkg")
            }
            else -> Unit // limpiar caché o abrir pantallas no se revierte
        }
    }

    private fun needElevated() = check(ready) { "Requiere el modo avanzado (Shizuku)" }

    private fun gamePackage(tweak: JSONObject): String =
        requireSafePackage(tweak.optString("target").ifEmpty { activeGame ?: error("No hay juego activo") })

    private fun parseGameMode(out: String): String = when {
        "performance" in out.lowercase() -> "performance"
        "battery" in out.lowercase() -> "battery"
        else -> "standard"
    }
}
