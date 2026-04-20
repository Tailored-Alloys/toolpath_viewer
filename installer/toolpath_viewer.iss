; ============================================================
; Toolpath Viewer - Inno Setup Installer Script
; Supports: Install, Uninstall, and Update (re-install over existing)
; ============================================================

#define MyAppName "Toolpath Viewer"
#define MyAppVersion "1.0.1"
#define MyAppPublisher "Tailored Alloys"
#define MyAppExeName "toolpath_viewer.exe"
#define MyAppURL "https://github.com/Tailored-Alloys/toolpath_viewer"

[Setup]
; Unique AppId - DO NOT change between versions (enables upgrade detection)
AppId={{E7A3F1B2-4D5C-6E8F-9A0B-1C2D3E4F5A6B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
; Allow user to skip Start Menu group creation
AllowNoIcons=yes
; Output installer settings
OutputDir=output
OutputBaseFilename=ToolpathViewer_Setup_{#MyAppVersion}
; Use the app icon for the installer
SetupIconFile=..\src\presentation\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
; Compression
Compression=lzma2/ultra64
SolidCompression=yes
; Modern look
WizardStyle=modern
; Require admin for Program Files install; use LowestAvailable to allow user-level too
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
; Minimum Windows version (Windows 10)
MinVersion=10.0
; Architecture - 64-bit only
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Enable update: allow installing over existing version without uninstalling first
UsePreviousAppDir=yes
; Close running instances before install/update
CloseApplications=force
CloseApplicationsFilter=*.exe
; Restart the application after silent update
RestartApplications=yes
; Show the version in Add/Remove Programs
VersionInfoVersion={#MyAppVersion}.0
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} Setup
VersionInfoProductName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "fileassoc_cli"; Description: "Associate .cli files with {#MyAppName}"; GroupDescription: "File associations:"; Flags: unchecked
Name: "fileassoc_ilt"; Description: "Associate .ilt files with {#MyAppName}"; GroupDescription: "File associations:"; Flags: unchecked

[Files]
; Main executable (built with cargo build --release)
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Icon file
Source: "..\src\presentation\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion
; Example files (optional)
Source: "..\example\*"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs
; README
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"; Tasks: desktopicon

[Registry]
; File association for .cli files
Root: HKA; Subkey: "Software\Classes\.cli\OpenWithProgids"; ValueType: string; ValueName: "ToolpathViewer.cli"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc_cli
Root: HKA; Subkey: "Software\Classes\ToolpathViewer.cli"; ValueType: string; ValueName: ""; ValueData: "CLI Toolpath File"; Flags: uninsdeletekey; Tasks: fileassoc_cli
Root: HKA; Subkey: "Software\Classes\ToolpathViewer.cli\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\icon.ico,0"; Tasks: fileassoc_cli
Root: HKA; Subkey: "Software\Classes\ToolpathViewer.cli\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_cli

; File association for .ilt files
Root: HKA; Subkey: "Software\Classes\.ilt\OpenWithProgids"; ValueType: string; ValueName: "ToolpathViewer.ilt"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc_ilt
Root: HKA; Subkey: "Software\Classes\ToolpathViewer.ilt"; ValueType: string; ValueName: ""; ValueData: "ILT Toolpath File"; Flags: uninsdeletekey; Tasks: fileassoc_ilt
Root: HKA; Subkey: "Software\Classes\ToolpathViewer.ilt\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\icon.ico,0"; Tasks: fileassoc_ilt
Root: HKA; Subkey: "Software\Classes\ToolpathViewer.ilt\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_ilt

[Run]
; Option to launch after install
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// ============================================================
// Update detection: if already installed, offer to update
// ============================================================
function InitializeSetup(): Boolean;
var
  InstalledVersion: String;
  ResultCode: Integer;
begin
  Result := True;

  // Check if previous version is installed
  if RegQueryStringValue(HKLM, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{E7A3F1B2-4D5C-6E8F-9A0B-1C2D3E4F5A6B}_is1',
    'DisplayVersion', InstalledVersion) or
     RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{E7A3F1B2-4D5C-6E8F-9A0B-1C2D3E4F5A6B}_is1',
    'DisplayVersion', InstalledVersion) then
  begin
    if InstalledVersion = '{#MyAppVersion}' then
    begin
      if MsgBox('{#MyAppName} version ' + InstalledVersion + ' is already installed.' + #13#10 +
                'Would you like to repair/reinstall it?',
                mbConfirmation, MB_YESNO) = IDNO then
      begin
        Result := False;
        Exit;
      end;
    end
    else
    begin
      if MsgBox('{#MyAppName} version ' + InstalledVersion + ' is currently installed.' + #13#10 +
                'Would you like to update to version {#MyAppVersion}?',
                mbConfirmation, MB_YESNO) = IDNO then
      begin
        Result := False;
        Exit;
      end;
    end;
  end;
end;
