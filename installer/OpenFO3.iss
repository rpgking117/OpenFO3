; OpenFO3's Windows installer script.
; by RPGKING The Mad Modder
;
; tools/release/release.py builds it with the release's version and folders:
;   ISCC /DAppVersion=1.0.2 /DSourceDir=<the OpenFO3 game folder> /DOutputDir=<dist> OpenFO3.iss
;
; It installs the game folder (OpenFO3.exe, the launcher, the readme, licences
; and notices) and Start menu / desktop shortcuts to the launcher. Uninstalling
; removes only those: saves (Documents\My Games\OpenFO3) and settings
; (%APPDATA%\OpenFO3) stay, and Fallout 3 itself is never touched.

#ifndef AppVersion
  #define AppVersion "1.0.2"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\OpenFO3"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

#define AppName "OpenFO3"
#define Publisher "RPGKING The Mad Modder"
#define LauncherExe "OpenFO3 Launcher.exe"

[Setup]
AppId={{C6359134-0A34-437D-B14F-3843858971B0}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#Publisher}
AppCopyright=Copyright (C) 2026 {#Publisher} and the nv-rs contributors
AppComments=An open-source engine for Fallout 3. Requires your own copy of Fallout 3.
VersionInfoCompany={#Publisher}
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} Setup
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; The welcome page shows the wizard picture.
DisableWelcomePage=no
; Per-user or for everyone, as the person installing chooses.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile={#SourceDir}\LICENSE.txt
InfoBeforeFile=before-install.txt
SetupIconFile=..\launcher\assets\openfo3.ico
UninstallDisplayIcon={app}\{#LauncherExe}
UninstallDisplayName={#AppName}
WizardStyle=modern
WizardImageFile=wizard.bmp,wizard-2x.bmp
WizardSmallImageFile=wizard-small.bmp,wizard-small-2x.bmp
OutputDir={#OutputDir}
OutputBaseFilename=OpenFO3-{#AppVersion}-setup
Compression=lzma2/max
SolidCompression=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#LauncherExe}"; Comment: "Play Fallout 3 on OpenFO3"
Name: "{autoprograms}\{#AppName} Readme"; Filename: "{app}\README.txt"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#LauncherExe}"; Comment: "Play Fallout 3 on OpenFO3"; Tasks: desktopicon

[Run]
Filename: "{app}\{#LauncherExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
