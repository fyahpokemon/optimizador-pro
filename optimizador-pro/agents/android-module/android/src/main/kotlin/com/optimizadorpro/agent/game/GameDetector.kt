package com.optimizadorpro.agent.game

import android.app.usage.UsageEvents
import android.app.usage.UsageStatsManager
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager

/** Juegos instalados y app en primer plano (requiere "Acceso de uso"). */
class GameDetector(private val ctx: Context) {
    private val pm = ctx.packageManager
    private val usm = ctx.getSystemService(UsageStatsManager::class.java)

    data class Game(val packageName: String, val label: String)

    /** Apps de launcher marcadas como juego por su desarrollador o presentes en el catálogo. */
    fun installedGames(catalog: Set<String> = emptySet()): List<Game> {
        val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        return pm.queryIntentActivities(launcher, PackageManager.MATCH_ALL)
            .map { it.activityInfo.applicationInfo }
            .distinctBy { it.packageName }
            .filter { isGame(it) || it.packageName in catalog }
            .map { Game(it.packageName, pm.getApplicationLabel(it).toString()) }
            .sortedBy { it.label.lowercase() }
    }

    fun isGamePackage(pkg: String, catalog: Set<String>): Boolean =
        pkg in catalog || runCatching { isGame(pm.getApplicationInfo(pkg, 0)) }.getOrDefault(false)

    fun label(pkg: String): String =
        runCatching { pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString() }.getOrDefault(pkg)

    @Suppress("DEPRECATION")
    private fun isGame(ai: ApplicationInfo): Boolean =
        ai.category == ApplicationInfo.CATEGORY_GAME || (ai.flags and ApplicationInfo.FLAG_IS_GAME) != 0

    fun usageAccessGranted(): Boolean {
        val now = System.currentTimeMillis()
        return usm.queryUsageStats(UsageStatsManager.INTERVAL_DAILY, now - 60_000, now).isNotEmpty()
    }

    /**
     * Paquete que pasó a primer plano más recientemente dentro de [windowMs], o null si no
     * hubo cambios en ese intervalo (el llamador conserva el último conocido).
     */
    fun foregroundPackage(windowMs: Long = 10_000): String? {
        val end = System.currentTimeMillis()
        val events = usm.queryEvents(end - windowMs, end)
        val e = UsageEvents.Event()
        var last: String? = null
        while (events.hasNextEvent()) {
            events.getNextEvent(e)
            if (e.eventType == UsageEvents.Event.ACTIVITY_RESUMED) last = e.packageName
        }
        return last
    }
}
