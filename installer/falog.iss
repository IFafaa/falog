; Windows installer for Falog, built by scripts/build-installer.ps1 with Inno Setup 6.
; Installs for the current user (no administrator rights), like scripts/install.ps1:
;   - falog.exe, falog-mcp.exe and the Visual C++ runtime DLLs in %LOCALAPPDATA%\Falog
;   - Start menu shortcut, optional desktop shortcut, optional launch at sign-in
;   - the assistant workspace in %USERPROFILE%\falog-assistant
;   - the MCP server registered with Claude Code when the `claude` command exists
; Uninstalling keeps the user's data in %USERPROFILE%\.falog.

#ifndef AppVersion
  #error Pass /DAppVersion=x.y.z (scripts/build-installer.ps1 does)
#endif
#ifndef BinDir
  #error Pass /DBinDir=<folder with falog.exe and falog-mcp.exe>
#endif
#ifndef CrtDir
  #error Pass /DCrtDir=<Microsoft.VC143.CRT folder>
#endif

[Setup]
AppId={{98711603-C8E6-4494-B1CA-4563E061D7DB}
AppName=Falog
AppVersion={#AppVersion}
AppVerName=Falog {#AppVersion}
AppPublisher=Falog
AppPublisherURL=https://github.com/IFafaa/falog
AppSupportURL=https://github.com/IFafaa/falog/issues
DefaultDirName={localappdata}\Falog
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\target\installer
OutputBaseFilename=Falog-Setup-{#AppVersion}
SetupIconFile=..\crates\falog-desktop\assets\icon\falog.ico
UninstallDisplayIcon={app}\falog.exe
UninstallDisplayName=Falog
LicenseFile=..\LICENSE
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; Falog keeps running in the background; let Setup close it before replacing the files.
CloseApplications=force
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[CustomMessages]
english.Autostart=Open Falog when I sign in to Windows
brazilianportuguese.Autostart=Abrir o Falog quando eu entrar no Windows
english.Mcp=Connect Falog to Claude Code (lets Claude read and file your tasks)
brazilianportuguese.Mcp=Conectar o Falog ao Claude Code (o Claude passa a ler e criar suas tarefas)
english.RegisteringMcp=Connecting Falog to Claude Code...
brazilianportuguese.RegisteringMcp=Conectando o Falog ao Claude Code...

[Tasks]
Name: "autostart"; Description: "{cm:Autostart}"
Name: "mcp"; Description: "{cm:Mcp}"; Check: ClaudeInstalled
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#BinDir}\falog.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\falog-mcp.exe"; DestDir: "{app}"; Flags: ignoreversion
; App-local Visual C++ runtime, so machines without the redistributable installed still start Falog.
Source: "{#CrtDir}\msvcp140.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#CrtDir}\vcruntime140.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#CrtDir}\vcruntime140_1.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\crates\falog-desktop\assets\THIRD-PARTY-NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assistant\CLAUDE.md"; DestDir: "{%USERPROFILE}\falog-assistant"; Flags: ignoreversion
Source: "..\assistant\.claude\settings.json"; DestDir: "{%USERPROFILE}\falog-assistant\.claude"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Falog"; Filename: "{app}\falog.exe"
Name: "{autodesktop}\Falog"; Filename: "{app}\falog.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Falog"; ValueData: """{app}\falog.exe"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{cmd}"; Parameters: "/c claude mcp remove --scope user falog >nul 2>&1 & claude mcp add --scope user falog -- ""{app}\falog-mcp.exe"""; StatusMsg: "{cm:RegisteringMcp}"; Flags: runhidden; Tasks: mcp
Filename: "{app}\falog.exe"; Description: "{cm:LaunchProgram,Falog}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{cmd}"; Parameters: "/c claude mcp remove --scope user falog"; Flags: runhidden; RunOnceId: "RemoveFalogMcp"; Check: ClaudeInstalled

[Code]
{ True when the Claude Code CLI is on PATH; the assistant dock and the MCP registration need it. }
function ClaudeInstalled: Boolean;
var
  ResultCode: Integer;
begin
  Result := Exec(ExpandConstant('{cmd}'), '/c where claude >nul 2>&1', '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
end;
