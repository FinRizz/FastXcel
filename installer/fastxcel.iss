#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

[Setup]
AppId={{BDA05BE3-C417-4C7C-82EB-FDE61096D541}
AppName=FastXcel
AppVersion={#MyAppVersion}
AppPublisher=FinRizz
AppPublisherURL=https://github.com/FinRizz/FastXcel
DefaultDirName={localappdata}\Programs\FastXcel
DefaultGroupName=FastXcel
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=fastxcel-setup
SetupIconFile=..\assets\fastxcel.ico
UninstallDisplayIcon={app}\fastxcel.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "..\target\release\fastxcel.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\FastXcel"; Filename: "{app}\fastxcel.exe"
Name: "{autodesktop}\FastXcel"; Filename: "{app}\fastxcel.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\fastxcel.exe"; Description: "Launch FastXcel"; Flags: nowait postinstall skipifsilent
