package com.optimizadorpro.agent

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import com.optimizadorpro.agent.game.GameModeService
import com.optimizadorpro.agent.oem.OemIntents
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Executors

/**
 * Puente Flutter ↔ Android.
 *
 * - Métodos `optimizador/android`: JSON en texto, mismo contrato que el núcleo en Rust.
 * - Eventos `optimizador/android/thermal`: estado térmico.
 * - Eventos `optimizador/android/session`: estado de la partida cada segundo.
 *
 * Lo que puede bloquear (Shizuku, dumpsys, disco) corre en un hilo de trabajo.
 */
class OptimizadorPlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
    private lateinit var ctx: Context
    private lateinit var rt: AgentRuntime
    private lateinit var methods: MethodChannel
    private lateinit var thermalEvents: EventChannel
    private lateinit var sessionEvents: EventChannel

    private val worker = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private var thermalListener: PowerManager.OnThermalStatusChangedListener? = null
    private var sessionListener: ((JSONObject) -> Unit)? = null

    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        ctx = binding.applicationContext
        rt = AgentRuntime.init(ctx)

        methods = MethodChannel(binding.binaryMessenger, "optimizador/android")
        methods.setMethodCallHandler(this)

        thermalEvents = EventChannel(binding.binaryMessenger, "optimizador/android/thermal")
        thermalEvents.setStreamHandler(object : EventChannel.StreamHandler {
            override fun onListen(args: Any?, sink: EventChannel.EventSink) {
                thermalListener = rt.scanner.addThermalListener { status -> sink.success(status) }
            }

            override fun onCancel(args: Any?) {
                thermalListener?.let(rt.scanner::removeThermalListener)
                thermalListener = null
            }
        })

        sessionEvents = EventChannel(binding.binaryMessenger, "optimizador/android/session")
        sessionEvents.setStreamHandler(object : EventChannel.StreamHandler {
            override fun onListen(args: Any?, sink: EventChannel.EventSink) {
                val l: (JSONObject) -> Unit = { st -> main.post { sink.success(st.toString()) } }
                sessionListener = l
                rt.sessions.addListener(l)
                sink.success(rt.sessions.state().toString())
            }

            override fun onCancel(args: Any?) {
                sessionListener?.let { rt.sessions.removeListener(it) }
                sessionListener = null
            }
        })
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        methods.setMethodCallHandler(null)
        thermalEvents.setStreamHandler(null)
        sessionEvents.setStreamHandler(null)
        sessionListener?.let { rt.sessions.removeListener(it) }
        // El runtime y el servicio siguen vivos: la partida no depende de la UI.
        worker.shutdown()
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        // Operaciones que deben ocurrir en el hilo principal (diálogos, actividades, servicio).
        when (call.method) {
            "requestElevation" -> return rt.elevated.requestPermission { ok -> main.post { result.success(ok) } }
            "openScreen" -> return result.success(OemIntents.open(ctx, call.argument<String>("screen")!!))
            "setAutoGameMode" -> {
                rt.settings.autoGameMode = call.argument<Boolean>("enabled") == true
                GameModeService.sync(ctx)
                return result.success(rt.settings.toJson().toString())
            }
        }
        worker.execute {
            try {
                val value = handle(call)
                main.post { result.success(value) }
            } catch (e: Exception) {
                main.post { result.error(e.javaClass.simpleName, e.message, null) }
            }
        }
    }

    private fun handle(call: MethodCall): Any? = when (call.method) {
        "configure" -> {
            call.argument<List<String>>("catalog")?.let { rt.settings.catalog = it.toSet() }
            call.argument<List<String>>("whitelist")?.let { rt.settings.whitelist = rt.settings.whitelist + it }
            call.argument<List<String>>("known_heavy")?.let { rt.settings.knownHeavy = it.toSet() }
            // Si el modo automático estaba activo y el sistema mató el servicio, se recupera.
            if (rt.settings.autoGameMode) main.post { GameModeService.sync(ctx) }
            rt.settings.toJson().toString()
        }
        "snapshot" -> rt.scanner.snapshot(rt.applier.activeGame).toString()
        "capabilities" -> JSONObject()
            .put("elevated", rt.elevated.status().name.lowercase())
            .put("usage_access", rt.games.usageAccessGranted())
            .put("aggressive_oem", OemIntents.hasAggressiveOem())
            .put("thermal_headroom", rt.scanner.thermalHeadroom() != null)
            .toString()
        "games" -> {
            val excluded = rt.settings.excludedGames
            JSONArray(
                rt.games.installedGames(rt.settings.catalog).map {
                    JSONObject().put("package", it.packageName).put("label", it.label).put("enabled", it.packageName !in excluded)
                },
            ).toString()
        }
        "batteryStatus" -> {
            val games = rt.games.installedGames(rt.settings.catalog).map { it.packageName to it.label }
            rt.battery.status(games).toString()
        }
        "settings" -> rt.settings.toJson().toString()
        "setGameEnabled" -> {
            rt.settings.setExcluded(call.argument<String>("package")!!, excluded = call.argument<Boolean>("enabled") != true)
            rt.settings.toJson().toString()
        }
        "apply" -> {
            val session = rt.journal.begin(call.argument<String>("label") ?: "one_click")
            val report = rt.journal.applyAll(session, JSONArray(call.argument<String>("tweaks")!!))
            report.put("session", session).toString()
        }
        "undo" -> rt.journal.undo(call.argument<Number>("session")!!.toLong())
        "sessionStart" -> {
            val pkg = call.argument<String>("package")!!
            val st = rt.sessions.start(pkg, call.argument<String>("label") ?: rt.games.label(pkg))
            main.post { GameModeService.sync(ctx) }
            st.toString()
        }
        "sessionState" -> rt.sessions.state().toString()
        "sessionStop" -> {
            val restored = rt.sessions.stop()
            main.post { GameModeService.sync(ctx) } // se detiene solo si no hay modo automático
            restored
        }
        "clearOwnCache" -> ctx.cacheDir.listFiles()
            ?.sumOf { f -> f.walkTopDown().sumOf { it.length() }.also { f.deleteRecursively() } } ?: 0L
        // NotImplementedError es un Error (no Exception): no lo capturaría el catch y cerraría la app.
        else -> throw UnsupportedOperationException("Método no implementado: ${call.method}")
    }
}
