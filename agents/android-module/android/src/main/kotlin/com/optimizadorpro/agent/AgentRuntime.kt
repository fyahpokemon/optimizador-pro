package com.optimizadorpro.agent

import android.content.Context
import com.optimizadorpro.agent.battery.BatteryInspector
import com.optimizadorpro.agent.game.GameDetector
import com.optimizadorpro.agent.game.GameSessionManager
import com.optimizadorpro.agent.sensors.DeviceScanner
import com.optimizadorpro.agent.settings.AgentSettings
import com.optimizadorpro.agent.shizuku.Elevated
import com.optimizadorpro.agent.shizuku.elevatedFor
import com.optimizadorpro.agent.tweaks.AndroidApplier
import com.optimizadorpro.agent.tweaks.TweakJournal

/**
 * Piezas compartidas por el plugin (UI abierta) y el servicio en primer plano (UI cerrada,
 * juego en pantalla). Durante la partida la interfaz de Flutter está en segundo plano y su
 * motor Dart puede quedar en pausa, así que todo lo que ocurre mientras se juega vive aquí.
 */
object AgentRuntime {
    @Volatile private var initialized = false

    lateinit var settings: AgentSettings; private set
    lateinit var elevated: Elevated; private set
    lateinit var scanner: DeviceScanner; private set
    lateinit var applier: AndroidApplier; private set
    lateinit var journal: TweakJournal; private set
    lateinit var games: GameDetector; private set
    lateinit var battery: BatteryInspector; private set
    lateinit var sessions: GameSessionManager; private set

    fun init(context: Context): AgentRuntime {
        if (initialized) return this
        synchronized(this) {
            if (initialized) return this
            val app = context.applicationContext
            settings = AgentSettings(app)
            elevated = elevatedFor(app)
            scanner = DeviceScanner(app, elevated)
            applier = AndroidApplier(app, elevated)
            journal = TweakJournal(app, applier)
            games = GameDetector(app)
            battery = BatteryInspector(app, elevated)
            sessions = GameSessionManager(app, this)
            // Partida interrumpida (el sistema mató la app durante el juego): deshacer al volver.
            journal.pending(GameSessionManager.GAME_LABEL).forEach { journal.undo(it) }
            initialized = true
        }
        return this
    }
}
