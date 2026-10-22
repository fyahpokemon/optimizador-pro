package com.optimizadorpro.agent.shizuku;

// Servicio que Shizuku ejecuta con la identidad "shell" (la misma que ADB).
interface IShellService {
    // Ejecuta un comando y devuelve stdout (stderr precedido de "ERR:" si falla).
    String exec(String command) = 1;

    // Código de transacción reservado por Shizuku para destruir el servicio.
    void destroy() = 16777114;
}
