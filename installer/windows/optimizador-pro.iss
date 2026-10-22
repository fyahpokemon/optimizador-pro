; Instalador de Optimizador Pro para Windows (Inno Setup 6).
; Lo compila .github/workflows/windows-exe.yml; las rutas de origen son relativas a dist\.
; Instala la app (Flutter) y el servicio con privilegios que aplica los ajustes.

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{7C3E2B1A-5D4F-4E8B-9A61-0F2C8D7E4B13}
AppName=Optimizador Pro
AppVersion={#AppVersion}
AppPublisher=Optimizador Pro
DefaultDirName={autopf}\Optimizador Pro
DefaultGroupName=Optimizador Pro
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=Optimizador-Pro-Setup
Compression=lzma2/max
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; El servicio necesita administrador para instalarse.
PrivilegesRequired=admin
WizardStyle=modern
UninstallDisplayIcon={app}\optimizador_pro.exe

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\..\dist\app\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\dist\service\optimizador-service.exe"; DestDir: "{app}\service"; Flags: ignoreversion
Source: "..\..\dist\service\bin\*"; DestDir: "{app}\service\bin"; Flags: ignoreversion skipifsourcedoesntexist recursesubdirs

[Icons]
Name: "{group}\Optimizador Pro"; Filename: "{app}\optimizador_pro.exe"
Name: "{autodesktop}\Optimizador Pro"; Filename: "{app}\optimizador_pro.exe"; Tasks: desktopicon

[Run]
; Registrar y arrancar el servicio. Si ya existía (actualización), se recrea apuntando al nuevo binario.
Filename: "{sys}\sc.exe"; Parameters: "stop OptimizadorPro"; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "delete OptimizadorPro"; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "create OptimizadorPro binPath= ""{app}\service\optimizador-service.exe"" start= auto DisplayName= ""Optimizador Pro"""; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "description OptimizadorPro ""Diagnóstico y ajustes de rendimiento de Optimizador Pro"""; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "failure OptimizadorPro reset= 86400 actions= restart/5000/restart/5000//0"; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "start OptimizadorPro"; Flags: runhidden waituntilterminated
Filename: "{app}\optimizador_pro.exe"; Description: "{cm:LaunchProgram,Optimizador Pro}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Al parar, el servicio deshace los cambios de una partida en curso.
Filename: "{sys}\sc.exe"; Parameters: "stop OptimizadorPro"; Flags: runhidden waituntilterminated; RunOnceId: "StopService"
Filename: "{sys}\sc.exe"; Parameters: "delete OptimizadorPro"; Flags: runhidden waituntilterminated; RunOnceId: "DeleteService"
