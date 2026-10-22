package com.optimizadorpro.agent.settings

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

/**
 * Preferencias persistentes. Se guardan en disco porque el servicio puede arrancar una
 * partida automáticamente sin que la parte de Flutter esté en marcha.
 */
class AgentSettings(context: Context) {
    private val prefs = context.getSharedPreferences("optimizador_agent", Context.MODE_PRIVATE)

    /** Activar el modo juego al detectar un juego en pantalla. */
    var autoGameMode: Boolean
        get() = prefs.getBoolean(K_AUTO, false)
        set(v) = prefs.edit().putBoolean(K_AUTO, v).apply()

    /** Juegos que el usuario ha desactivado (por defecto se optimizan todos los detectados). */
    var excludedGames: Set<String>
        get() = prefs.getStringSet(K_EXCLUDED, emptySet())!!.toSet()
        set(v) = prefs.edit().putStringSet(K_EXCLUDED, v).apply()

    /** Catálogo de juegos conocidos (enviado por la app desde `game_catalog.json`). */
    var catalog: Set<String>
        get() = prefs.getStringSet(K_CATALOG, emptySet())!!.toSet()
        set(v) = prefs.edit().putStringSet(K_CATALOG, v).apply()

    /** Apps que nunca se frenan (teclado, servicios de Google, voz de Discord…). */
    var whitelist: Set<String>
        get() = prefs.getStringSet(K_WHITELIST, DEFAULT_WHITELIST)!!.toSet()
        set(v) = prefs.edit().putStringSet(K_WHITELIST, v).apply()

    /** Lastre conocido: se frena durante la partida aunque no aparezca en las estadísticas. */
    var knownHeavy: Set<String>
        get() = prefs.getStringSet(K_HEAVY, emptySet())!!.toSet()
        set(v) = prefs.edit().putStringSet(K_HEAVY, v).apply()

    fun setExcluded(pkg: String, excluded: Boolean) {
        excludedGames = if (excluded) excludedGames + pkg else excludedGames - pkg
    }

    fun toJson(): JSONObject = JSONObject()
        .put("auto_game_mode", autoGameMode)
        .put("excluded_games", JSONArray(excludedGames.sorted()))

    private companion object {
        const val K_AUTO = "auto_game_mode"
        const val K_EXCLUDED = "excluded_games"
        const val K_CATALOG = "catalog"
        const val K_WHITELIST = "whitelist"
        const val K_HEAVY = "known_heavy"

        val DEFAULT_WHITELIST = setOf(
            "com.android.systemui", "com.google.android.gms", "com.google.android.inputmethod.latin",
            "com.samsung.android.honeyboard", "com.discord", "com.google.android.apps.messaging",
            "com.android.phone", "com.google.android.dialer",
        )
    }
}
