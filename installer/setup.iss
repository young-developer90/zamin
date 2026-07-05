; Lion Programming Language Installer for Windows x64
; Inno Setup script — https://jrsoftware.org/isinfo.php

#define MyAppName "Lion"
#define MyAppVersion "1.5.5"
#define MyAppPublisher "Lion Language"
#define MyAppURL "https://github.com/young-developer90/lion"
#define MyAppExeName "lion.exe"

[Setup]
AppId={{B8A7C3E1-9F4D-4A2E-BC5D-8F6E2D1A3C7B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
VersionInfoVersion={#MyAppVersion}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
InfoBeforeFile=..\README.md
OutputDir=.
OutputBaseFilename=Lion-{#MyAppVersion}-x64
SetupIconFile=..\assets\lion-logo.ico
WizardImageFile=..\assets\lion-logo.png
WizardSmallImageFile=..\assets\lion-logo.png
UninstallDisplayIcon={app}\bin\{#MyAppExeName}
UninstallDisplayName={#MyAppName} {#MyAppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
InternalCompressLevel=ultra64
WizardStyle=modern dynamic
DisableWelcomePage=no
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
ChangesEnvironment=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"; Flags: checkedonce
Name: "quicklaunchicon"; Description: "Create a &Quick Launch icon"; GroupDescription: "Additional icons:"; Flags: unchecked
Name: "addtopath"; Description: "Add Lion to &PATH (system-wide)"; GroupDescription: "Environment:"; Flags: checkedonce
Name: "assoclion"; Description: "Associate &.lion files with Lion"; GroupDescription: "File associations:"; Flags: checkedonce
Name: "installmodules"; Description: "Install C extension &development headers"; GroupDescription: "Development:"; Flags: unchecked

[Files]
; Main binary
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}\bin"; Flags: ignoreversion; Tasks: ; Check: Is64BitInstallMode
Source: "..\target\release\lion-lsp.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\target\release\lion.pdb"; DestDir: "{app}\bin"; Flags: ignoreversion skipifsourcedoesntexist

; C extension API header
Source: "..\include\lion.h"; DestDir: "{app}\include"; Flags: ignoreversion; Tasks: installmodules

; Example C extension
Source: "..\modules\example.c"; DestDir: "{app}\modules"; Flags: ignoreversion; Tasks: installmodules

; Documentation
Source: "..\README.md"; DestDir: "{app}\doc"; Flags: isreadme
Source: "..\TUTORIAL.md"; DestDir: "{app}\doc"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}\doc"; Flags: ignoreversion

; Examples
Source: "..\examples\*.lion"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs

; Benchmarks
Source: "..\benchmarks\*.lion"; DestDir: "{app}\benchmarks"; Flags: ignoreversion
Source: "..\benchmarks\*.py"; DestDir: "{app}\benchmarks"; Flags: ignoreversion
Source: "..\benchmarks\*.bat"; DestDir: "{app}\benchmarks"; Flags: ignoreversion

; Tests
Source: "..\tests\*.lion"; DestDir: "{app}\tests"; Flags: ignoreversion recursesubdirs

; Brand assets
Source: "..\assets\lion-logo.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\lion-logo.png"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\lion-logo.svg"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\lion-file-icon.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\lion-file-icon.png"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\lion-file-icon.svg"; DestDir: "{app}\assets"; Flags: ignoreversion

; VS Code extension
Source: "..\vscode-lion\*"; DestDir: "{app}\vscode-lion"; Flags: ignoreversion recursesubdirs; Excludes: "node_modules"

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\bin\{#MyAppExeName}"; Parameters: "repl"; WorkingDir: "{app}"; Comment: "Start Lion REPL"
Name: "{group}\Lion Examples"; Filename: "{app}\examples"; WorkingDir: "{app}\examples"; Comment: "Browse example scripts"
Name: "{group}\Lion Documentation"; Filename: "{app}\doc\TUTORIAL.md"; Comment: "Open the tutorial"
Name: "{group}\Browse Source on GitHub"; Filename: "{#MyAppURL}"; Comment: "Open the GitHub repository"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{commondesktop}\{#MyAppName} REPL"; Filename: "{app}\bin\{#MyAppExeName}"; Parameters: "repl"; WorkingDir: "{app}"; Tasks: desktopicon; Comment: "Start Lion REPL"

[Registry]
; File association for .lion files
Root: HKA; Subkey: "Software\Classes\.lion"; ValueType: string; ValueName: ""; ValueData: "LionScript"; Flags: uninsdeletevalue; Tasks: assoclion
Root: HKA; Subkey: "Software\Classes\LionScript"; ValueType: string; ValueName: ""; ValueData: "Lion Script"; Flags: uninsdeletekey; Tasks: assoclion
Root: HKA; Subkey: "Software\Classes\LionScript\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\assets\lion-file-icon.ico"; Tasks: assoclion
Root: HKA; Subkey: "Software\Classes\LionScript\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\{#MyAppExeName}"" run ""%1"""; Tasks: assoclion
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".lion"; ValueData: ""; Tasks: assoclion

; PATH environment variable (system-wide)
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "PATH"; ValueData: "{olddata};{app}\bin"; Check: NeedsAddPath('{app}\bin'); Tasks: addtopath

[Run]
Filename: "{app}\bin\{#MyAppExeName}"; Parameters: "version"; WorkingDir: "{app}"; Description: "Verify installation"; Flags: postinstall nowait skipifsilent shellexec
Filename: "{app}\bin\{#MyAppExeName}"; Parameters: "repl"; WorkingDir: "{app}"; Description: "Start the REPL"; Flags: postinstall nowait skipifsilent unchecked shellexec
Filename: "https://github.com/young-developer90/lion"; Description: "View documentation online"; Flags: postinstall nowait skipifsilent unchecked shellexec

[UninstallRun]
Filename: "{app}\bin\{#MyAppExeName}"; Parameters: "version"; WorkingDir: "{app}"; Flags: runhidden; RunOnceId: "Lion_UninstallVersionCheck"

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'PATH', OrigPath) then
  begin
    Result := True;
    Exit;
  end;
  Result := Pos(LowerCase(Param), LowerCase(OrigPath)) = 0;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ErrorCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    if WizardIsTaskSelected('assoclion') then
    begin
      // Notify Windows about file association changes
      ShellExec('open', 'cmd.exe', '/c assoc .lion=LionScript', '', SW_HIDE, ewNoWait, ErrorCode);
    end;
  end;
end;

procedure RegisterPreviousData(PreviousDataKey: Integer);
begin
  // Store whether PATH was modified so uninstall can clean it
  if WizardIsTaskSelected('addtopath') then
    SetPreviousData(PreviousDataKey, 'AddedToPath', 'True')
  else
    SetPreviousData(PreviousDataKey, 'AddedToPath', 'False');
end;

function InitializeUninstall: Boolean;
var
  Path: string;
  AppBinDir: string;
  NewPath: string;
  AddedToPath: string;
  P: Integer;
  Entry: string;
begin
  Result := True;
  AddedToPath := GetPreviousData('AddedToPath', '');
  if AddedToPath = 'True' then
  begin
    AppBinDir := ExpandConstant('{app}\bin');
    if RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'PATH', Path) then
    begin
      NewPath := '';
      while Path <> '' do
      begin
        P := Pos(';', Path);
        if P = 0 then
        begin
          Entry := Trim(Path);
          Path := '';
        end else
        begin
          Entry := Trim(Copy(Path, 1, P - 1));
          Delete(Path, 1, P);
        end;
        if CompareText(Entry, AppBinDir) <> 0 then
        begin
          if NewPath <> '' then
            NewPath := NewPath + ';';
          NewPath := NewPath + Entry;
        end;
      end;
      if NewPath <> Path then
        RegWriteExpandStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'PATH', NewPath);
    end;
  end;
end;
