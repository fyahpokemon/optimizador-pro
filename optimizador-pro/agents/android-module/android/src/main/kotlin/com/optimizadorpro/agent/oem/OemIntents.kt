package com.optimizadorpro.agent.oem

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings

/**
 * Pantallas del sistema a las que la app guía al usuario. Las de los fabricantes cambian
 * entre versiones de su capa: se prueban en orden y se cae a la pantalla estándar.
 * Mantener sincronizado con `data/oem_screens.json` (actualizable sin publicar versión).
 */
object OemIntents {

    private val AUTOSTART: Map<String, List<ComponentName>> = mapOf(
        "xiaomi" to listOf(
            ComponentName("com.miui.securitycenter", "com.miui.permcenter.autostart.AutoStartManagementActivity"),
        ),
        "redmi" to listOf(
            ComponentName("com.miui.securitycenter", "com.miui.permcenter.autostart.AutoStartManagementActivity"),
        ),
        "poco" to listOf(
            ComponentName("com.miui.securitycenter", "com.miui.permcenter.autostart.AutoStartManagementActivity"),
        ),
        "huawei" to listOf(
            ComponentName("com.huawei.systemmanager", "com.huawei.systemmanager.startupmgr.ui.StartupNormalAppListActivity"),
            ComponentName("com.huawei.systemmanager", "com.huawei.systemmanager.optimize.process.ProtectActivity"),
        ),
        "honor" to listOf(
            ComponentName("com.hihonor.systemmanager", "com.hihonor.systemmanager.startupmgr.ui.StartupNormalAppListActivity"),
        ),
        "oppo" to listOf(
            ComponentName("com.coloros.safecenter", "com.coloros.safecenter.permission.startup.StartupAppListActivity"),
            ComponentName("com.oplus.battery", "com.oplus.powermanager.fuelgaue.PowerUsageModelActivity"),
        ),
        "realme" to listOf(
            ComponentName("com.oplus.battery", "com.oplus.powermanager.fuelgaue.PowerUsageModelActivity"),
        ),
        "oneplus" to listOf(
            ComponentName("com.oneplus.security", "com.oneplus.security.chainlaunch.view.ChainLaunchAppListActivity"),
        ),
        "vivo" to listOf(
            ComponentName("com.iqoo.secure", "com.iqoo.secure.ui.phoneoptimize.BgStartUpManager"),
            ComponentName("com.vivo.permissionmanager", "com.vivo.permissionmanager.activity.BgStartUpManagerActivity"),
        ),
        "samsung" to listOf(
            ComponentName("com.samsung.android.lool", "com.samsung.android.sm.battery.ui.BatteryActivity"),
            ComponentName("com.samsung.android.sm_cn", "com.samsung.android.sm.ui.battery.BatteryActivity"),
        ),
        "asus" to listOf(
            ComponentName("com.asus.mobilemanager", "com.asus.mobilemanager.powersaver.PowerSaverSettings"),
        ),
    )

    /** Intents para una pantalla, del más específico al estándar. */
    fun forScreen(ctx: Context, screen: String): List<Intent> = when (screen) {
        "oem_autostart" -> AUTOSTART[Build.MANUFACTURER.lowercase()].orEmpty()
            .map { Intent().setComponent(it) } +
            Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)
        // Política de Play: no se pide la exención directa; se abre la lista para que el usuario elija.
        "battery_optimization" -> listOf(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
        "battery_saver" -> listOf(Intent(Settings.ACTION_BATTERY_SAVER_SETTINGS))
        "storage" -> listOf(Intent(Settings.ACTION_INTERNAL_STORAGE_SETTINGS))
        "usage_access" -> listOf(Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS))
        "app_details" -> listOf(
            Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${ctx.packageName}")),
        )
        "game_dashboard" -> listOf(Intent("android.settings.GAME_SETTINGS"), Intent(Settings.ACTION_SETTINGS))
        else -> listOf(Intent(Settings.ACTION_SETTINGS))
    }.map { it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) }

    /** Abre la primera pantalla que exista en este dispositivo. */
    fun open(ctx: Context, screen: String): Boolean =
        forScreen(ctx, screen).any { intent -> runCatching { ctx.startActivity(intent) }.isSuccess }

    fun hasAggressiveOem(): Boolean = Build.MANUFACTURER.lowercase() in AUTOSTART.keys
}
