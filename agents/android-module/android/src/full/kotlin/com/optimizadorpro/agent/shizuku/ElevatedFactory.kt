package com.optimizadorpro.agent.shizuku

import android.content.ComponentName
import android.content.Context
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.IBinder
import rikka.shizuku.Shizuku
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Sabor `full`: Shizuku con el patrón UserService (proceso propio con identidad "shell"). */
object ElevatedFactory {
    fun create(context: Context): Elevated = ShizukuElevated(context.applicationContext)
}

private class ShizukuElevated(private val context: Context) : Elevated {
    private val requestCode = 7341
    @Volatile private var service: IShellService? = null
    private var bound = CountDownLatch(1)

    private val args = Shizuku.UserServiceArgs(ComponentName(context.packageName, ShellService::class.java.name))
        .daemon(false)
        .processNameSuffix("shell")
        .debuggable(false)
        .version(1)

    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, binder: IBinder?) {
            if (binder != null && binder.pingBinder()) {
                service = IShellService.Stub.asInterface(binder)
                bound.countDown()
            }
        }

        override fun onServiceDisconnected(name: ComponentName) {
            service = null
            bound = CountDownLatch(1)
        }
    }

    override fun status(): Elevated.Status = when {
        !Shizuku.pingBinder() -> Elevated.Status.NOT_RUNNING
        Shizuku.isPreV11() -> Elevated.Status.UNSUPPORTED
        Shizuku.checkSelfPermission() != PackageManager.PERMISSION_GRANTED -> Elevated.Status.NEEDS_PERMISSION
        else -> Elevated.Status.READY
    }

    override fun requestPermission(onResult: (Boolean) -> Unit) {
        if (status() == Elevated.Status.READY) return onResult(true)
        if (status() != Elevated.Status.NEEDS_PERMISSION) return onResult(false)
        val listener = object : Shizuku.OnRequestPermissionResultListener {
            override fun onRequestPermissionResult(code: Int, grantResult: Int) {
                if (code != requestCode) return
                Shizuku.removeRequestPermissionResultListener(this)
                onResult(grantResult == PackageManager.PERMISSION_GRANTED)
            }
        }
        Shizuku.addRequestPermissionResultListener(listener)
        Shizuku.requestPermission(requestCode)
    }

    private fun ensureBound(): IShellService {
        service?.let { return it }
        check(status() == Elevated.Status.READY) { "Shizuku no está listo" }
        Shizuku.bindUserService(args, connection)
        check(bound.await(5, TimeUnit.SECONDS)) { "Shizuku no respondió" }
        return service ?: error("Servicio de shell no disponible")
    }

    override fun exec(command: String): String {
        val out = ensureBound().exec(command)
        if (out.startsWith("ERR:")) throw IllegalStateException(out.removePrefix("ERR:").trim())
        return out
    }

    override fun release() {
        if (service != null) runCatching { Shizuku.unbindUserService(args, connection, true) }
        service = null
    }
}
