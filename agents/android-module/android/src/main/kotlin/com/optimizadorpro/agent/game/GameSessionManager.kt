package com.optimizadorpro.agent.game

import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import android.os.PowerManager
import com.optimizadorpro.agent.AgentRuntime
import com.optimizadorpro.agent.shizuku.Elevated
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit

/**
 * Partida en curso, gestionada en nativo (la UI de Flutter está en segundo plano).
 *
 * Al empezar: modo juego del sistema, juego exento de Doze y apps lastre frenadas (con
 * Shizuku); todo queda en el journal. Cada segundo: frametimes, estado térmico y
 * [ThermalGovernor]. Al terminar: se deshace todo.
 */
class GameSessionManager(private val ctx: Context, private val rt: AgentRuntime) {
    private val pm = ctx.getSystemService(PowerManager::class.java)
    private val executor = Executors.newSingleThreadScheduledExecutor { r -> Thread(r, "game-session") }
    private val listeners = CopyOnWriteArrayList<(JSONObject) -> Unit>()

    private class Session(
        val pkg: String,
        val label: String,
        val auto: Boolean,
        val journalId: Long,
        val startedMs: Long,
        val governor: ThermalGovernor,
        val sampler: FrameSampler,
        val window: FrameWindow = FrameWindow(10.0),
        var applied: Int = 0,
        var capTip: Int? = null,
        /** Por qué cambió el tope por última vez (para explicarlo en la UI). */
        var capReason: ThermalGovernor.Reason? = null,
        var stats: FrameStats? = null,
        var thermal: Int = PowerManager.THERMAL_STATUS_NONE,
        var tick: ScheduledFuture<*>? = null,
    )

    @Volatile private var session: Session? = null
    @Volatile private var lastSummary: JSONObject? = null

    fun addListener(l: (JSONObject) -> Unit) = listeners.add(l)
    fun removeListener(l: (JSONObject) -> Unit) = listeners.remove(l)

    fun isActive(): Boolean = session != null

    /** Empieza (o cambia a) una partida. Bloquea hasta aplicar los ajustes iniciales. */
    fun start(pkg: String, label: String = rt.games.label(pkg), auto: Boolean = false): JSONObject =
        executor.submit<JSONObject> {
            session?.let { finish(it) }
            val ram = ctx.getSystemService(ActivityManager::class.java).let { am ->
                ActivityManager.MemoryInfo().also { am.getMemoryInfo(it) }.totalMem / 1_073_741_824.0
            }
            val refresh = rt.scanner.hardwareInfo().optInt("refresh_hz", 60)
            val s = Session(
                pkg = pkg,
                label = label,
                auto = auto,
                journalId = rt.journal.begin(GAME_LABEL),
                startedMs = System.currentTimeMillis(),
                governor = ThermalGovernor(ThermalGovernor.profileCapFor(ram, refresh)),
                sampler = FrameSampler(rt.elevated, pkg),
            )
            rt.applier.activeGame = pkg
            session = s
            lastSummary = null
            s.applied = rt.journal.applyAll(s.journalId, initialTweaks(pkg)).optInt("applied")
            // Con Shizuku, el tope inicial del perfil se aplica de verdad; sin él, se recomienda.
            s.governor.cap?.let { applyCap(s, it) }
            s.tick = executor.scheduleWithFixedDelay({ tick(s) }, 1, 1, TimeUnit.SECONDS)
            publish()
            state()
        }.get()

    /** Termina la partida y deshace sus cambios. Devuelve cuántos se restauraron. */
    fun stop(): Int = executor.submit<Int> { session?.let { finish(it) } ?: 0 }.get()

    fun state(): JSONObject {
        val s = session ?: return JSONObject().put("active", false).put("summary", lastSummary ?: JSONObject.NULL)
        return JSONObject()
            .put("active", true)
            .put("package", s.pkg)
            .put("label", s.label)
            .put("auto", s.auto)
            .put("started_ms", s.startedMs)
            .put("applied", s.applied)
            .put("stats", s.stats?.toJson() ?: JSONObject.NULL)
            .put("fps_cap", s.governor.cap ?: JSONObject.NULL)
            .put("cap_tip", s.capTip ?: JSONObject.NULL)
            .put("cap_reason", s.capReason?.name?.lowercase() ?: JSONObject.NULL)
            .put("thermal", com.optimizadorpro.agent.sensors.DeviceScanner.thermalName(s.thermal))
            .put("battery_temp_c", batteryTempC())
            .put("measures_fps", s.sampler.available())
    }

    private fun initialTweaks(pkg: String): JSONArray {
        val tweaks = JSONArray()
        if (rt.elevated.status() != Elevated.Status.READY) return tweaks
        tweaks.put(JSONObject().put("kind", "enable_game_mode").put("target", pkg))
        tweaks.put(JSONObject().put("kind", "exempt_from_battery_opt").put("package", pkg))
        val skip = rt.settings.whitelist + pkg + ctx.packageName
        (rt.scanner.recentBackgroundApps() + rt.settings.knownHeavy)
            .distinct()
            .filter { it !in skip && isInstalled(it) }
            .take(MAX_THROTTLED)
            .forEach { tweaks.put(JSONObject().put("kind", "throttle_background").put("pid", 0).put("name", it)) }
        return tweaks
    }

    private fun tick(s: Session) {
        if (session !== s) return
        runCatching {
            s.sampler.drain().forEach { s.window.push(it.toDouble()) }
            s.stats = s.window.stats()
            s.thermal = pm.currentThermalStatus
            s.governor.step(System.currentTimeMillis(), s.thermal, rt.scanner.thermalHeadroom(), s.stats)
                ?.let {
                    s.capReason = it.reason
                    applyCap(s, it.cap)
                }
        }
        publish()
    }

    /** Fija el tope por el sistema si hay Shizuku; si no, lo deja como recomendación. */
    private fun applyCap(s: Session, cap: Int?) {
        if (rt.elevated.status() == Elevated.Status.READY) {
            val tweak = JSONObject().put("kind", "set_fps_cap").put("target", s.pkg).put("fps", cap ?: 0)
            val ok = rt.journal.applyAll(s.journalId, JSONArray().put(tweak)).optInt("applied") > 0
            s.capTip = if (ok) null else cap
        } else {
            s.capTip = cap
        }
    }

    private fun finish(s: Session): Int {
        s.tick?.cancel(false)
        session = null
        rt.applier.activeGame = null
        val restored = rt.journal.undo(s.journalId)
        lastSummary = JSONObject()
            .put("label", s.label)
            .put("minutes", (System.currentTimeMillis() - s.startedMs) / 60_000)
            .put("restored", restored)
        publish()
        return restored
    }

    private fun publish() {
        val st = state()
        listeners.forEach { runCatching { it(st) } }
    }

    private fun isInstalled(pkg: String) = runCatching { ctx.packageManager.getApplicationInfo(pkg, 0) }.isSuccess

    private fun batteryTempC(): Double {
        val b = ctx.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        return (b?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) ?: 0) / 10.0
    }

    companion object {
        const val GAME_LABEL = "game"
        private const val MAX_THROTTLED = 8
    }
}
