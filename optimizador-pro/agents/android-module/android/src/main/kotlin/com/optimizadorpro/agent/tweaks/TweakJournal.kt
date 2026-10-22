package com.optimizadorpro.agent.tweaks

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * Journal de reversión en Android (mismo comportamiento que `core/journal` en Rust):
 * se guarda el valor previo ANTES de aplicar; deshacer recorre la sesión en orden inverso.
 * Persistente en un archivo JSON para sobrevivir a que el sistema mate la app.
 */
class TweakJournal(ctx: Context, private val applier: AndroidApplier) {
    private val file = File(ctx.filesDir, "journal.json")
    private val lock = Any()

    private fun load(): JSONObject =
        runCatching { JSONObject(file.readText()) }.getOrElse { JSONObject().put("next", 1).put("sessions", JSONObject()) }

    private fun save(root: JSONObject) {
        val tmp = File(file.parentFile, "journal.json.tmp")
        tmp.writeText(root.toString())
        tmp.renameTo(file) // escritura atómica
    }

    fun begin(label: String): Long = synchronized(lock) {
        val root = load()
        val id = root.getLong("next")
        root.put("next", id + 1)
        root.getJSONObject("sessions").put(
            id.toString(),
            JSONObject().put("label", label).put("started", System.currentTimeMillis()).put("entries", JSONArray()),
        )
        save(root)
        id
    }

    /** Aplica y devuelve el informe {applied, failed:[{tweak, error}]}. */
    fun applyAll(session: Long, tweaks: JSONArray): JSONObject {
        var applied = 0
        val failed = JSONArray()
        for (i in 0 until tweaks.length()) {
            val t = tweaks.getJSONObject(i)
            try {
                val prev = applier.readCurrent(t)
                val index = record(session, t, prev)
                try {
                    applier.apply(t)
                    applied++
                } catch (e: Exception) {
                    forget(session, index)
                    throw e
                }
            } catch (e: Exception) {
                failed.put(JSONObject().put("tweak", t).put("error", e.message ?: e.javaClass.simpleName))
            }
        }
        return JSONObject().put("applied", applied).put("failed", failed)
    }

    fun undo(session: Long): Int = synchronized(lock) {
        val root = load()
        val s = root.getJSONObject("sessions").optJSONObject(session.toString()) ?: return 0
        val entries = s.getJSONArray("entries")
        var restored = 0
        for (i in entries.length() - 1 downTo 0) {
            val e = entries.getJSONObject(i)
            if (e.optBoolean("reverted")) continue
            runCatching { applier.restore(e.getJSONObject("tweak"), e.getJSONObject("previous")) }
                .onSuccess { restored++ }
            e.put("reverted", true) // si el proceso ya no existe, no hay nada que restaurar
        }
        save(root)
        restored
    }

    /** Sesiones con cambios sin deshacer y una etiqueta dada (p. ej. partidas interrumpidas). */
    fun pending(label: String): List<Long> = synchronized(lock) {
        val sessions = load().getJSONObject("sessions")
        sessions.keys().asSequence().filter { k ->
            val s = sessions.getJSONObject(k)
            val entries = s.getJSONArray("entries")
            s.getString("label") == label &&
                (0 until entries.length()).any { !entries.getJSONObject(it).optBoolean("reverted") }
        }.map { it.toLong() }.sorted().toList()
    }

    private fun record(session: Long, tweak: JSONObject, previous: JSONObject): Int = synchronized(lock) {
        val root = load()
        val entries = root.getJSONObject("sessions").getJSONObject(session.toString()).getJSONArray("entries")
        entries.put(JSONObject().put("tweak", tweak).put("previous", previous).put("reverted", false))
        save(root)
        entries.length() - 1
    }

    private fun forget(session: Long, index: Int) = synchronized(lock) {
        val root = load()
        root.getJSONObject("sessions").getJSONObject(session.toString()).getJSONArray("entries").remove(index)
        save(root)
    }
}
