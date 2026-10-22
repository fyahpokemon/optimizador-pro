# Instala el servicio de Windows para desarrollo (ejecutar como administrador).
# En producción lo instala el paquete MSIX (extensión desktop6:Service).
param(
    [string]$Exe = "$PSScriptRoot\..\target\release\optimizador-service.exe"
)
$ErrorActionPreference = "Stop"
$name = "OptimizadorPro"

if (Get-Service -Name $name -ErrorAction SilentlyContinue) {
    Stop-Service $name -ErrorAction SilentlyContinue
    sc.exe delete $name | Out-Null
    Start-Sleep -Seconds 1
}

$bin = (Resolve-Path $Exe).Path
New-Service -Name $name -BinaryPathName "`"$bin`"" -DisplayName "Optimizador Pro" `
    -Description "Diagnóstico y ajustes de rendimiento de Optimizador Pro" -StartupType Automatic | Out-Null
# Reiniciar si falla (el servicio deshace las partidas pendientes al arrancar).
sc.exe failure $name reset= 86400 actions= restart/5000/restart/5000/""/0 | Out-Null
Start-Service $name
Write-Host "Servicio $name instalado y en marcha."
