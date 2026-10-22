package com.optimizadorpro.agent.game

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import com.optimizadorpro.agent.AgentRuntime
import org.json.JSONObject

/**
 * Servicio en primer plano del modo juego. Hace dos cosas:
 * 1. Con el modo automático activado, vigila qué app está en pantalla y empieza/termina la
 *    partida solo ([ForegroundWatcher]).
 * 2. Mientras hay partida, mantiene vivo el proceso (y con él la reversión) y muestra una
 *    notificación discreta con el estado y un botón "Terminar".
 *
 * Se detiene solo cuando no hay modo automático ni partida.
 */
class GameModeService : Service() {
    private lateinit var rt: AgentRuntime
    private val main = Handler(Looper.getMainLooper())
    private var watcher: ForegroundWatcher? = null
    private var lastText: String? = null
    @Volatile private var destroyed = false
    private val onState: (JSONObject) -> Unit = { st -> main.post { render(st) } }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        rt = AgentRuntime.init(this)
        getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel(CHANNEL, "Modo juego", NotificationManager.IMPORTANCE_LOW).apply {
                description = "Estado del modo juego mientras juegas"
                setShowBadge(false)
                setSound(null, null)
            },
        )
        goForeground(build("Modo juego", "Preparando…", withStop = false))
        rt.sessions.addListener(onState)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP_SESSION -> Thread {
                rt.sessions.stop()
                main.post { stopIfIdle() }
            }.start()
            else -> {
                syncWatcher()
                render(rt.sessions.state())
                stopIfIdle()
            }
        }
        // Si el sistema mata el servicio con el modo automático activo, que lo recree.
        return if (rt.settings.autoGameMode) START_STICKY else START_NOT_STICKY
    }

    override fun onDestroy() {
        destroyed = true
        rt.sessions.removeListener(onState)
        watcher?.stop()
        watcher = null
        super.onDestroy()
    }

    private fun syncWatcher() {
        val wanted = rt.settings.autoGameMode
        if (wanted && watcher == null) {
            watcher = ForegroundWatcher(
                ctx = this,
                detector = rt.games,
                isTarget = { pkg ->
                    pkg !in rt.settings.excludedGames && rt.games.isGamePackage(pkg, rt.settings.catalog)
                },
                onGameEnter = { pkg -> rt.sessions.start(pkg, auto = true) },
                onGameExit = { pkg ->
                    val st = rt.sessions.state()
                    if (st.optBoolean("auto") && st.optString("package") == pkg) rt.sessions.stop()
                },
            ).also { it.start() }
        } else if (!wanted && watcher != null) {
            watcher?.stop()
            watcher = null
        }
    }

    private fun stopIfIdle() {
        if (!rt.settings.autoGameMode && !rt.sessions.isActive()) stopSelf()
    }

    private fun render(st: JSONObject) {
        if (destroyed) return
        val (title, text) = notificationText(st)
        if (text == lastText) return
        lastText = text
        getSystemService(NotificationManager::class.java)
            .notify(NOTIFICATION_ID, build(title, text, withStop = st.optBoolean("active")))
    }

    private fun build(title: String, text: String, withStop: Boolean): Notification {
        val open = packageManager.getLaunchIntentForPackage(packageName)?.let {
            PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }
        val builder = Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentTitle(title)
            .setContentText(text)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(Notification.CATEGORY_SERVICE)
            .setContentIntent(open)
        if (withStop) {
            val stop = PendingIntent.getForegroundService(
                this,
                1,
                Intent(this, GameModeService::class.java).setAction(ACTION_STOP_SESSION),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            builder.addAction(Notification.Action.Builder(null, "Terminar", stop).build())
        }
        return builder.build()
    }

    private fun goForeground(n: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(NOTIFICATION_ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(NOTIFICATION_ID, n)
        }
    }

    companion object {
        private const val CHANNEL = "game_mode"
        private const val NOTIFICATION_ID = 41
        const val ACTION_STOP_SESSION = "com.optimizadorpro.agent.STOP_SESSION"

        /** Arranca o actualiza el servicio (llamar desde la app en primer plano). */
        fun sync(ctx: Context) {
            ctx.startForegroundService(Intent(ctx, GameModeService::class.java))
        }

        /** Textos de la notificación en lenguaje cotidiano (función pura, probada en la JVM). */
        fun notificationText(st: JSONObject): Pair<String, String> {
            if (!st.optBoolean("active")) return "Modo juego automático" to "Esperando a que abras un juego"
            val title = "Modo juego · ${st.optString("label")}"
            val tip = st.opt("cap_tip")
            if (tip is Int) return title to "Para evitar calor, limita el juego a $tip FPS"
            val stats = st.optJSONObject("stats")
            val text = when {
                stats == null && !st.optBoolean("measures_fps") -> "Cuidando la temperatura y las apps en segundo plano"
                stats == null -> "Midiendo…"
                stats.optBoolean("stable") -> "${stats.optDouble("avg_fps").toInt()} FPS estables"
                else -> "${stats.optDouble("avg_fps").toInt()} FPS · ajustando"
            }
            return title to text
        }
    }
}
