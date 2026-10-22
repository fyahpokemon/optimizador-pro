package com.optimizadorpro.agent.battery

import android.app.ActivityManager
import android.app.usage.UsageStatsManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import android.os.Build
import android.os.PowerManager
import com.optimizadorpro.agent.oem.OemIntents
import com.optimizadorpro.agent.sensors.DeviceScanner
import com.optimizadorpro.agent.shizuku.Elevated
import org.json.JSONArray
import org.json.JSONObject

/**
 * Todo lo que decide si el modo juego puede funcionar en segundo plano y si la batería está
 * frenando el rendimiento. Solo APIs públicas: no necesita permisos especiales.
 */
class BatteryInspector(private val ctx: Context, private val elevated: Elevated) {
    private val pm = ctx.getSystemService(PowerManager::class.java)
    private val am = ctx.getSystemService(ActivityManager::class.java)
    private val usm = ctx.getSystemService(UsageStatsManager::class.java)

    /** [games]: paquetes y nombres de los juegos detectados, para saber cuáles están exentos. */
    fun status(games: List<Pair<String, String>>): JSONObject {
        val b = ctx.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        val level = b?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
        val scale = b?.getIntExtra(BatteryManager.EXTRA_SCALE, 100) ?: 100
        val plugged = (b?.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0) ?: 0) != 0

        val gamesJson = JSONArray()
        games.forEach { (pkg, label) ->
            gamesJson.put(
                JSONObject()
                    .put("package", pkg)
                    .put("label", label)
                    .put("battery_exempt", pm.isIgnoringBatteryOptimizations(pkg)),
            )
        }

        return JSONObject()
            .put("level_pct", if (level >= 0 && scale > 0) level * 100 / scale else JSONObject.NULL)
            .put("charging", plugged)
            .put("temp_c", (b?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) ?: 0) / 10.0)
            .put("power_save", pm.isPowerSaveMode)
            // Si es false, el sistema puede dormir el servicio del modo juego.
            .put("app_exempt", pm.isIgnoringBatteryOptimizations(ctx.packageName))
            // "Restringida" en Ajustes > Batería: el sistema no deja trabajar en segundo plano.
            .put("background_restricted", am.isBackgroundRestricted)
            .put("standby_bucket", bucketName(usm.appStandbyBucket))
            .put("manufacturer", Build.MANUFACTURER.lowercase())
            .put("aggressive_oem", OemIntents.hasAggressiveOem())
            .put("thermal", DeviceScanner.thermalName(pm.currentThermalStatus))
            .put(
                "thermal_headroom",
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    pm.getThermalHeadroom(10).takeIf { !it.isNaN() } ?: JSONObject.NULL
                } else {
                    JSONObject.NULL
                },
            )
            .put("elevated", elevated.status().name.lowercase())
            .put("games", gamesJson)
    }

    companion object {
        fun bucketName(bucket: Int): String = when (bucket) {
            UsageStatsManager.STANDBY_BUCKET_ACTIVE -> "active"
            UsageStatsManager.STANDBY_BUCKET_WORKING_SET -> "working_set"
            UsageStatsManager.STANDBY_BUCKET_FREQUENT -> "frequent"
            UsageStatsManager.STANDBY_BUCKET_RARE -> "rare"
            UsageStatsManager.STANDBY_BUCKET_RESTRICTED -> "restricted"
            else -> "unknown"
        }
    }
}
