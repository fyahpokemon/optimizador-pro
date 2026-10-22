package com.optimizadorpro.agent.shizuku

import android.content.Context

/** Sabor `play`: sin funciones elevadas (política de Google Play). */
object ElevatedFactory {
    fun create(@Suppress("UNUSED_PARAMETER") context: Context): Elevated = object : Elevated {
        override fun status() = Elevated.Status.UNSUPPORTED
        override fun requestPermission(onResult: (Boolean) -> Unit) = onResult(false)
        override fun exec(command: String): String =
            throw UnsupportedOperationException("Funciones elevadas no disponibles en esta versión")
        override fun release() {}
    }
}
