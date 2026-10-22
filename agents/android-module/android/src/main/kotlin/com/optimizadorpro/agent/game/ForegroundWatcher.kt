package com.optimizadorpro.agent.game

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.PowerManager
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit

/**
 * Detecta cuándo un juego pasa a primer plano y cuándo se sale de él.
 *
 * Consulta los eventos de uso cada [periodMs] solo con la pantalla encendida: con la
 * pantalla apagada no hay nada que optimizar y así el vigilante no gasta batería.
 * (Un AccessibilityService sería instantáneo, pero Google Play no lo permite para esto.)
 */
class ForegroundWatcher(
    private val ctx: Context,
    private val detector: GameDetector,
    /** ¿Este paquete es un juego que el usuario quiere optimizar? */
    private val isTarget: (String) -> Boolean,
    private val onGameEnter: (pkg: String) -> Unit,
    private val onGameExit: (pkg: String) -> Unit,
    private val periodMs: Long = 2_000,
) {
    private val executor = Executors.newSingleThreadScheduledExecutor { r -> Thread(r, "foreground-watcher") }
    private var task: ScheduledFuture<*>? = null
    private var lastForeground: String? = null
    private var activeGame: String? = null

    private val screenReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            when (intent.action) {
                Intent.ACTION_SCREEN_ON -> schedule()
                Intent.ACTION_SCREEN_OFF -> pause()
            }
        }
    }

    fun start() {
        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_SCREEN_ON)
            addAction(Intent.ACTION_SCREEN_OFF)
        }
        // Emisiones del sistema: NOT_EXPORTED las sigue recibiendo y evita el error de lint.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            ctx.registerReceiver(screenReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            ctx.registerReceiver(screenReceiver, filter)
        }
        if (ctx.getSystemService(PowerManager::class.java).isInteractive) schedule()
    }

    fun stop() {
        runCatching { ctx.unregisterReceiver(screenReceiver) }
        pause()
        executor.shutdownNow()
    }

    @Synchronized private fun schedule() {
        if (task != null || executor.isShutdown) return
        task = executor.scheduleWithFixedDelay(::poll, 0, periodMs, TimeUnit.MILLISECONDS)
    }

    @Synchronized private fun pause() {
        task?.cancel(false)
        task = null
    }

    private fun poll() {
        val fg = runCatching { detector.foregroundPackage(periodMs * 5) }.getOrNull() ?: lastForeground ?: return
        if (fg == lastForeground) return
        lastForeground = fg
        if (fg == ctx.packageName) return // abrir Optimizador Pro durante la partida no la termina

        val current = activeGame
        when {
            current == null && isTarget(fg) -> {
                activeGame = fg
                onGameEnter(fg)
            }
            current != null && fg != current -> {
                activeGame = null
                onGameExit(current)
                if (isTarget(fg)) {
                    activeGame = fg
                    onGameEnter(fg)
                }
            }
        }
    }
}
