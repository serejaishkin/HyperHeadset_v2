[Setup]
AppName=HyperX NGENUITY Open
AppVersion=0.2.0
AppPublisher=serezaiskin-cell
DefaultDirName={autopf}\HyperX NGENUITY Open
DefaultGroupName=HyperX NGENUITY Open
OutputDir=.\output
OutputBaseFilename=HyperX-NGENUITY-Open-Setup
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
WizardStyle=modern

[Files]
Source: "..\..\target\release\hyperx_ngenuity_open.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\lang\*.lang"; DestDir: "{app}\lang"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\HyperX NGENUITY Open"; Filename: "{app}\hyperx_ngenuity_open.exe"
Name: "{autodesktop}\HyperX NGENUITY Open"; Filename: "{app}\hyperx_ngenuity_open.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional icons:"
Name: "autostart"; Description: "&Start with Windows"; GroupDescription: "Autostart:"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "HyperXNGENUITYOpen"; ValueData: """{app}\hyperx_ngenuity_open.exe"""; Tasks: autostart

[Run]
Filename: "{app}\hyperx_ngenuity_open.exe"; Description: "Launch HyperX NGENUITY Open"; Flags: postinstall skipifsilent nowait

[UninstallDelete]
Type: filesandordirs; Name: "{app}\lang"
