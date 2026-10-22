package com.optimizadorpro.agent.sensors

import android.app.ActivityManager
import android.app.usage.UsageStatsManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import android.os.Build
import android.os.Environment
import android.os.PowerManager
import android.os.StatFs
import android.hardware.display.DisplayManager
import android.view.Display
import com.optimizadorpro.agent.shizuku.Elevated
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * Construye la instantánea con el contrato JSON de `model::Snapshot` (núcleo en Rust).
 *
 * Desde Android 8 una app no puede leer `/proc/stat` ni la CPU de otros procesos. Sin
 * elevación se usan: estado y margen térmico, frecuencias por núcleo, memoria del sistema y
 * estadísticas de uso. Con Shizuku se añade `dumpsys cpuinfo` (CPU por proceso).
 */
class DeviceScanner(private val ctx: Context, private val elevated: Elevated) {
    private val pm = ctx.getSystemService(PowerManager::class.java)
    private val am = ctx.getSystemService(ActivityManager::class.java)
    private val usm = ctx.getSystemService(UsageStatsManager::class.java)

    fun hardwareInfo(): JSONObject {
        val mem = ActivityManager.MemoryInfo().also { am.getMemoryInfo(it) }
        return JSONObject()
            .put("cpu_model", socName())
            .put("logical_cores", Runtime.getRuntime().availableProcessors())
            .put("ram_total_mb", mem.totalMem / 1_048_576)
            .put("gpu_model", JSONObject.NULL)
            .put("gpu_vram_mb", JSONObject.NULL)
            .put("storage_is_ssd", true)
            .put("has_battery", true)
            .put("refresh_hz", refreshRateHz() ?: JSONObject.NULL)
    }

    /** Instantánea completa. [foregroundPackage] marca el juego activo como primer plano. */
    fun snapshot(foregroundPackage: String? = null): JSONObject {
        val mem = ActivityManager.MemoryInfo().also { am.getMemoryInfo(it) }
        val stat = StatFs(Environment.getDataDirectory().path)
        val battery = ctx.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        val plugged = (battery?.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0) ?: 0) != 0
        val cpu = if (elevated.status() == Elevated.Status.READY) CpuInfo.read(elevated) else null

        val processes = JSONArray()
        cpu?.processes?.forEach { p ->
            processes.put(
                JSONObject()
                    .put("pid", p.pid)
                    .put("name", p.name)
                    .put("cpu_pct", p.cpuPct)
                    .put("ram_mb", 0)
                    .put("is_foreground", p.name == foregroundPackage)
                    .put("is_system", p.isSystem)
            )
        }

        return JSONObject()
            .put("platform", "android")
            .put("hw", hardwareInfo())
            .put("cpu_load_pct", cpu?.totalPct ?: 0.0)
            .put("cpu_freq_ratio", cpuFreqRatio() ?: JSONObject.NULL)
            .put("gpu_load_pct", JSONObject.NULL)
            .put("ram_used_pct", (1.0 - mem.availMem.toDouble() / mem.totalMem) * 100.0)
            .put("storage_free_pct", stat.availableBytes.toDouble() / stat.totalBytes * 100.0)
            .put("thermal", thermalName(pm.currentThermalStatus))
            .put("thermal_headroom", thermalHeadroom() ?: JSONObject.NULL)
            .put("on_battery", !plugged)
            .put("power_plan", JSONObject.NULL)
            .put("power_save_mode", pm.isPowerSaveMode)
            .put("game_mode_on", JSONObject.NULL)
            .put("ignoring_battery_opt", pm.isIgnoringBatteryOptimizations(ctx.packageName))
            .put("processes", processes)
            .put("temp_bytes", ctx.cacheDir.walkTopDown().filter { it.isFile }.sumOf { it.length() })
    }

    /** Previsión a 10 s; 1.0 = el sistema empezará a limitar. Requiere API 30. */
    fun thermalHeadroom(): Float? =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            pm.getThermalHeadroom(10).takeIf { !it.isNaN() }
        } else null

    fun addThermalListener(onChange: (String) -> Unit): PowerManager.OnThermalStatusChangedListener {
        val l = PowerManager.OnThermalStatusChangedListener { onChange(thermalName(it)) }
        pm.addThermalStatusListener(ctx.mainExecutor, l)
        return l
    }

    fun removeThermalListener(l: PowerManager.OnThermalStatusChangedListener) =
        pm.removeThermalStatusListener(l)

    /** Media de frecuencia actual / máxima por núcleo; null si el fabricante bloquea sysfs. */
    fun cpuFreqRatio(): Double? = runCatching {
        File("/sys/devices/system/cpu").listFiles { f -> f.name.matches(Regex("cpu\\d+")) }
            .orEmpty()
            .mapNotNull { core ->
                val cur = File(core, "cpufreq/scaling_cur_freq").readText().trim().toDouble()
                val max = File(core, "cpufreq/cpuinfo_max_freq").readText().trim().toDouble()
                if (max > 0) cur / max else null
            }
            .takeIf { it.isNotEmpty() }
            ?.average()
    }.getOrNull()

    /** Apps con servicio en primer plano largo en la última hora (lastre probable). */
    fun recentBackgroundApps(): List<String> {
        val end = System.currentTimeMillis()
        val start = end - 3_600_000
        return runCatching {
            usm.queryUsageStats(UsageStatsManager.INTERVAL_DAILY, start, end)
                .filter { it.packageName != ctx.packageName && it.lastTimeUsed > start }
                .filter { Build.VERSION.SDK_INT >= 29 && it.totalTimeForegroundServiceUsed > 600_000 }
                .sortedByDescending { it.totalTimeForegroundServiceUsed }
                .map { it.packageName }
        }.getOrDefault(emptyList())
    }

    /** El contexto de aplicación no tiene pantalla (`ctx.display` lanza): se usa DisplayManager. */
    private fun refreshRateHz(): Int? = runCatching {
        ctx.getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)?.refreshRate?.toInt()
    }.getOrNull()

    companion object {
        fun thermalName(status: Int): String = when (status) {
            PowerManager.THERMAL_STATUS_NONE -> "normal"
            PowerManager.THERMAL_STATUS_LIGHT -> "light"
            PowerManager.THERMAL_STATUS_MODERATE -> "moderate"
            PowerManager.THERMAL_STATUS_SEVERE -> "severe"
            else -> "critical"
        }
    }
}

private fun socName(): String =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) "${Build.SOC_MANUFACTURER} ${Build.SOC_MODEL}".trim()
    else "${Build.BRAND} ${Build.HARDWARE}".trim()
