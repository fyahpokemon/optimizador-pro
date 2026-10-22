package com.optimizadorpro.agent.shizuku

import android.content.Context

/**
 * Acceso a comandos de nivel ADB ("shell"). En el sabor `full` lo implementa Shizuku;
 * en el sabor `play` no existe y todas las funciones elevadas se ocultan en la UI.
 */
interface Elevated {
    enum class Status { UNSUPPORTED, NOT_RUNNING, NEEDS_PERMISSION, READY }

    fun status(): Status

    /** Pide permiso al usuario (diálogo de Shizuku). El resultado llega por [onResult]. */
    fun requestPermission(onResult: (Boolean) -> Unit)

    /** Ejecuta un comando como "shell". Lanza excepción si no está [Status.READY]. */
    fun exec(command: String): String

    fun release()
}

/** Instancia según el sabor de compilación (ver `src/full` y `src/play`). */
fun elevatedFor(context: Context): Elevated = ElevatedFactory.create(context)

/** Valida nombres de paquete antes de interpolarlos en un comando de shell. */
private val PACKAGE_RE = Regex("^[A-Za-z][A-Za-z0-9_]*(\\.[A-Za-z0-9_]+)+$")

fun requireSafePackage(pkg: String): String {
    require(PACKAGE_RE.matches(pkg)) { "Nombre de paquete no válido: $pkg" }
    return pkg
}
