; AQBot Inno Setup 脚本
; 用于 Tauri v2 Windows 桌面应用

#define MyAppName "AQBot"
#define MyAppVersion "0.0.108"
#define MyAppPublisher "AQBot-Desktop"
#define MyAppURL "https://github.com/AQBot-Desktop/AQBot"
#define MyAppExeName "AQBot.exe"

[Setup]
AppId={{B4F1C8A2-3D5E-4F7A-9B2C-1D6E8F0A3C7B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=D:\aq\AQBot\src-tauri\target\release\bundle\innosetup
OutputBaseFilename=AQBot_v{#MyAppVersion}_Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
SetupIconFile=D:\aq\AQBot\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
DisableProgramGroupPage=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "zh"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "D:\aq\AQBot\src-tauri\target\release\AQBot.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "D:\aq\AQBot\src-tauri\target\release\WebView2Loader.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "D:\aq\AQBot\src-tauri\icons\icon.ico"; DestDir: "{app}\resources"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: postinstall nowait skipifsilent unchecked

[Code]
function InitializeSetup: Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  Exec('taskkill', '/f /im AQBot.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;
