; Oteryn Windows client installer (CLIENT-INSTALLER-0, docs/architecture/
; CLIENT-INSTALLER-0_WINDOWS_CLIENT_INSTALLER_CONTRACT_CANDIDATE.md). Per-user, no elevation.
;
; The release payload (oteryn-client.exe, client.env, packages.json) is not installed by
; [Files]: PrepareToInstall verifies or stages it into releases\<release_id>[~<n>], and after
; Setup installed its tracked files ssPostInstall activates it with one MoveFileExW rename of
; current.txt (section 2.1 steps 1-3); a failed activation exits with code 20. The transaction mutex
; Global\OterynClientSetup-<SID> and the client mutex Global\OterynClient-<SID> are handled in
; [Code] because their names are computed at run time.
;
; Compile (ISCC 6.7.3) with every define below, for example:
;   ISCC /DReleaseId=0.1.0+dev.ci.g1a2b3c4d5e6f /DClientVersion=0.1.0 /DChannel=dev
;        /DPayloadDir=<dir> /DLauncherPath=<exe> /DOutputDir=<dir>
;        /DClientExeSha256=<hex> /DClientEnvSha256=<hex> /DPackagesSha256=<hex> oteryn-client.iss
;
; Signing (section 6): sign.ps1 is the one hook. Inno Setup aborts a compile whose SignTool
; produces no signature, so SignTool is wired only when the signing release job defines it
; (/DSignTool=<name> together with ISCC /S<name>=...sign.ps1 $f); unsigned builds call
; sign.ps1 explicitly, where it no-ops.
;
; Command-line parameters besides the standard Inno Setup ones:
;   /EXPECTRELEASE=<id> /EXPECTCHANNEL=<channel> /EXPECTVERSION=<version>
;       refuse to run unless the embedded identity equals each given value (the updater passes
;       the channel pointer's values; section 5).
;   /RELAUNCH
;       after a successful install, start the launcher with --after-setup; it starts the client
;       only after this setup process has released the transaction mutex.

#ifndef ReleaseId
  #error ReleaseId is not defined
#endif
#ifndef ClientVersion
  #error ClientVersion is not defined
#endif
#ifndef Channel
  #error Channel is not defined
#endif
#ifndef PayloadDir
  #error PayloadDir is not defined
#endif
#ifndef LauncherPath
  #error LauncherPath is not defined
#endif
#ifndef OutputDir
  #error OutputDir is not defined
#endif
#ifndef ClientExeSha256
  #error ClientExeSha256 is not defined
#endif
#ifndef ClientEnvSha256
  #error ClientEnvSha256 is not defined
#endif
#ifndef PackagesSha256
  #error PackagesSha256 is not defined
#endif
#if Pos(ClientVersion + "+" + Channel + ".", ReleaseId) != 1
  #error ReleaseId does not start with ClientVersion+Channel.
#endif
#if Pos("~", ReleaseId) != 0
  #error ReleaseId must not contain ~
#endif
#if Len(ReleaseId) > 50
  #error ReleaseId is longer than 50 bytes
#endif
#if Len(ClientExeSha256) != 64 || Len(ClientEnvSha256) != 64 || Len(PackagesSha256) != 64
  #error a payload SHA-256 is not 64 hex digits
#endif

[Setup]
; One fixed AppId for every release and channel (section 2).
AppId={{9355097F-B319-4FEC-9314-3661537B6A22}
AppName=Oteryn
AppVersion={#ReleaseId}
AppVerName=Oteryn {#ReleaseId}
VersionInfoVersion={#ClientVersion}
DefaultDirName={localappdata}\Programs\Oteryn
DisableDirPage=yes
DisableProgramGroupPage=yes
DirExistsWarning=no
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=no
RestartApplications=no
SolidCompression=no
SetupLogging=yes
UninstallDisplayName=Oteryn
UninstallDisplayIcon={app}\oteryn-launcher.exe
OutputDir={#OutputDir}
OutputBaseFilename=oteryn-client-{#ReleaseId}-x86_64-setup
#ifdef SignTool
SignTool={#SignTool}
SignedUninstaller=yes
#endif

[Files]
Source: "{#LauncherPath}"; DestDir: "{app}"; DestName: "oteryn-launcher.exe"; Flags: ignoreversion
Source: "{#SourcePath}\..\THIRD-PARTY-FONTS.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\oteryn-client.exe"; DestDir: "{tmp}"; Flags: dontcopy
Source: "{#PayloadDir}\client.env"; DestDir: "{tmp}"; Flags: dontcopy
Source: "{#PayloadDir}\packages.json"; DestDir: "{tmp}"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\Oteryn"; Filename: "{app}\oteryn-launcher.exe"

[UninstallDelete]
Type: filesandordirs; Name: "{app}\releases"
Type: files; Name: "{app}\current.txt"
Type: files; Name: "{app}\current.txt.new"

[Code]
const
  ReleaseId = '{#ReleaseId}';
  ClientVersion = '{#ClientVersion}';
  Channel = '{#Channel}';
  ClientExeSha256 = '{#ClientExeSha256}';
  ClientEnvSha256 = '{#ClientEnvSha256}';
  PackagesSha256 = '{#PackagesSha256}';

  SYNCHRONIZE = $00100000;
  TOKEN_QUERY = $0008;
  TOKEN_USER_CLASS = 1;
  ERROR_FILE_NOT_FOUND = 2;
  ERROR_ALREADY_EXISTS = 183;
  GENERIC_WRITE = $40000000;
  CREATE_ALWAYS = 2;
  OPEN_EXISTING = 3;
  INVALID_HANDLE_VALUE = $FFFFFFFF;
  MOVEFILE_REPLACE_EXISTING = 1;
  MOVEFILE_WRITE_THROUGH = 8;
  ACTIVATION_FAILED_EXIT_CODE = 20;

var
  UserSid: String;
  SetupMutex: THandle;
  PreviousRelease, TargetRelease, ActivationError: String;

function GetCurrentProcess: THandle;
  external 'GetCurrentProcess@kernel32.dll stdcall';
function OpenProcessToken(ProcessHandle: THandle; DesiredAccess: DWORD; var TokenHandle: THandle): Boolean;
  external 'OpenProcessToken@advapi32.dll stdcall';
function GetTokenInformation(TokenHandle: THandle; InformationClass: Integer; Information: AnsiString;
  InformationLength: DWORD; var ReturnLength: DWORD): Boolean;
  external 'GetTokenInformation@advapi32.dll stdcall';
function CreateMutexW(MutexAttributes: DWORD; InitialOwner: BOOL; Name: String): THandle;
  external 'CreateMutexW@kernel32.dll stdcall';
function OpenMutexW(DesiredAccess: DWORD; InheritHandle: BOOL; Name: String): THandle;
  external 'OpenMutexW@kernel32.dll stdcall';
function CloseHandle(Handle: THandle): Boolean;
  external 'CloseHandle@kernel32.dll stdcall';
function CreateFileW(FileName: String; DesiredAccess, ShareMode, SecurityAttributes,
  CreationDisposition, FlagsAndAttributes: DWORD; TemplateFile: THandle): THandle;
  external 'CreateFileW@kernel32.dll stdcall';
function WriteFile(Handle: THandle; Buffer: AnsiString; BytesToWrite: DWORD;
  var BytesWritten: DWORD; Overlapped: DWORD): Boolean;
  external 'WriteFile@kernel32.dll stdcall';
function FlushFileBuffers(Handle: THandle): Boolean;
  external 'FlushFileBuffers@kernel32.dll stdcall';
function MoveFileExW(ExistingFileName, NewFileName: String; Flags: DWORD): Boolean;
  external 'MoveFileExW@kernel32.dll stdcall';

procedure Fail(const Message: String);
begin
  Log(Message);
  SuppressibleMsgBox(Message, mbCriticalError, MB_OK, IDOK);
end;

{ --- Current user SID ----------------------------------------------------------------------- }

function SidByte(const Buffer: AnsiString; Offset, Index: Integer): Int64;
begin
  Result := Ord(Buffer[Offset + Index + 1]);
end;

{ Formats the SID that follows the TOKEN_USER header as S-1-<authority>-<subauthority>... }
function CurrentUserSidString: String;
var
  Token: THandle;
  Buffer: AnsiString;
  Length_: DWORD;
  Offset, Count, I: Integer;
  Authority, Value: Int64;
begin
  if not OpenProcessToken(GetCurrentProcess, TOKEN_QUERY, Token) then
    RaiseException('OpenProcessToken failed: ' + SysErrorMessage(DLLGetLastError));
  try
    SetLength(Buffer, 512);
    Length_ := 0;
    if not GetTokenInformation(Token, TOKEN_USER_CLASS, Buffer, 512, Length_) then
      RaiseException('GetTokenInformation failed: ' + SysErrorMessage(DLLGetLastError));
  finally
    CloseHandle(Token);
  end;
  { TOKEN_USER is a SID pointer and a DWORD, padded to 16 bytes in a 64-bit process. }
  if Ord(Buffer[9]) = 1 then
    Offset := 8
  else
    Offset := 16;
  Count := Ord(Buffer[Offset + 2]);
  if (SidByte(Buffer, Offset, 0) <> 1) or (Count < 1) or (Count > 15) or
     (Offset + 8 + 4 * Count > Length_) then
    RaiseException('unexpected token user SID');
  Authority := 0;
  for I := 2 to 7 do
    Authority := Authority * 256 + SidByte(Buffer, Offset, I);
  if Authority > $FFFFFFFF then
    RaiseException('unsupported SID identifier authority');
  Result := 'S-1-' + IntToStr(Authority);
  for I := 0 to Count - 1 do begin
    Value := SidByte(Buffer, Offset, 8 + 4 * I) +
             SidByte(Buffer, Offset, 9 + 4 * I) * 256 +
             SidByte(Buffer, Offset, 10 + 4 * I) * 65536 +
             SidByte(Buffer, Offset, 11 + 4 * I) * 16777216;
    Result := Result + '-' + IntToStr(Value);
  end;
end;

{ --- Mutexes (section 2.1, Concurrency) --------------------------------------------------- }

function SetupMutexName: String;
begin
  Result := 'Global\OterynClientSetup-' + UserSid;
end;

function ClientMutexName: String;
begin
  Result := 'Global\OterynClient-' + UserSid;
end;

{ Creates the transaction mutex and holds it until this process exits; refuses if it exists. }
function TakeSetupMutex: Boolean;
var
  Error: Integer;
begin
  Result := False;
  try
    UserSid := CurrentUserSidString;
  except
    Fail('Oteryn: cannot determine the current user: ' + GetExceptionMessage);
    Exit;
  end;
  SetupMutex := CreateMutexW(0, False, SetupMutexName);
  Error := DLLGetLastError;
  if SetupMutex = 0 then begin
    Fail('Oteryn: cannot create the install mutex: ' + SysErrorMessage(Error));
    Exit;
  end;
  if Error = ERROR_ALREADY_EXISTS then begin
    CloseHandle(SetupMutex);
    SetupMutex := 0;
    Fail('Another Oteryn install or uninstall is running. Try again when it has finished.');
    Exit;
  end;
  Result := True;
end;

{ Whether any process holds the named mutex. Any error other than "not found" counts as held. }
function MutexHeld(const Name: String): Boolean;
var
  Handle: THandle;
begin
  Handle := OpenMutexW(SYNCHRONIZE, False, Name);
  if Handle <> 0 then begin
    CloseHandle(Handle);
    Result := True;
  end else
    Result := DLLGetLastError <> ERROR_FILE_NOT_FOUND;
end;

{ --- Command line ------------------------------------------------------------------------- }

function FindParam(const Prefix: String; var Value: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if SameText(Copy(ParamStr(I), 1, Length(Prefix)), Prefix) then begin
      Value := Copy(ParamStr(I), Length(Prefix) + 1, MaxInt);
      Result := True;
    end;
end;

function HasFlag(const Flag: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if SameText(ParamStr(I), Flag) then
      Result := True;
end;

{ A given /EXPECT...= value must equal the embedded one exactly. }
function ExpectationMet(const Prefix, Embedded, What: String): Boolean;
var
  Expected: String;
begin
  Result := True;
  if FindParam(Prefix, Expected) and (Expected <> Embedded) then begin
    Fail('This installer contains Oteryn ' + What + ' ' + Embedded + ', not the expected ' + Expected + '.');
    Result := False;
  end;
end;

{ --- Release directories (section 2.1) ---------------------------------------------------- }

function IsDigits(const Value: String): Boolean;
var
  I: Integer;
begin
  Result := Length(Value) > 0;
  for I := 1 to Length(Value) do
    if (Value[I] < '0') or (Value[I] > '9') then
      Result := False;
end;

{ Whether Name is <release_id> (Copy 0) or <release_id>~<n> (Copy n >= 1) of ReleaseId. }
function IsCopyOfRelease(const Name: String; var CopyNumber: Integer): Boolean;
var
  Suffix: String;
begin
  Result := False;
  if Name = ReleaseId then begin
    CopyNumber := 0;
    Result := True;
  end else if Copy(Name, 1, Length(ReleaseId) + 1) = ReleaseId + '~' then begin
    Suffix := Copy(Name, Length(ReleaseId) + 2, MaxInt);
    if IsDigits(Suffix) and (Suffix[1] <> '0') and (Length(Suffix) <= 9) then begin
      CopyNumber := StrToInt(Suffix);
      Result := True;
    end;
  end;
end;

function ReleaseOf(const Name: String): String;
var
  Tilde: Integer;
begin
  Tilde := Pos('~', Name);
  if Tilde = 0 then
    Result := Name
  else
    Result := Copy(Name, 1, Tilde - 1);
end;

{ The directory name current.txt holds, or '' when it is missing, malformed or dangling. }
function ReadPointer(const AppDir, Releases: String): String;
var
  Contents: AnsiString;
  Value: String;
begin
  Result := '';
  if not LoadStringFromFile(AppDir + '\current.txt', Contents) then
    Exit;
  Value := Contents;
  if Copy(Value, Length(Value) - 1, 2) = #13#10 then
    Value := Copy(Value, 1, Length(Value) - 2)
  else if Copy(Value, Length(Value), 1) = #10 then
    Value := Copy(Value, 1, Length(Value) - 1);
  if (Value = '') or (Length(Value) > 64) or (Pos('\', Value) > 0) or (Pos('/', Value) > 0) or
     (Pos(':', Value) > 0) or (Pos('..', Value) > 0) or (Value[1] = '.') then
    Exit;
  if DirExists(Releases + '\' + Value) then
    Result := Value;
end;

{ The names of the directories directly under Releases. }
function ListDirectories(const Releases: String): TArrayOfString;
var
  FindRec: TFindRec;
  Count: Integer;
begin
  Count := 0;
  SetArrayLength(Result, 0);
  if FindFirst(Releases + '\*', FindRec) then begin
    try
      repeat
        if ((FindRec.Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0) and
           (FindRec.Name <> '.') and (FindRec.Name <> '..') then begin
          SetArrayLength(Result, Count + 1);
          Result[Count] := FindRec.Name;
          Count := Count + 1;
        end;
      until not FindNext(FindRec);
    finally
      FindClose(FindRec);
    end;
  end;
end;

function HashMatches(const FileName, Expected: String): Boolean;
begin
  try
    Result := SameText(GetSHA256OfFile(FileName), Expected);
  except
    Log('Cannot hash ' + FileName + ': ' + GetExceptionMessage);
    Result := False;
  end;
end;

{ A release directory is valid when it holds exactly the three payload files with the embedded
  SHA-256 values and nothing else. }
function ReleaseDirectoryValid(const Dir: String): Boolean;
var
  FindRec: TFindRec;
  Files: Integer;
begin
  Result := True;
  Files := 0;
  if FindFirst(Dir + '\*', FindRec) then begin
    try
      repeat
        if (FindRec.Name = '.') or (FindRec.Name = '..') then
          Continue;
        if ((FindRec.Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0) or
           ((FindRec.Name <> 'oteryn-client.exe') and (FindRec.Name <> 'client.env') and
            (FindRec.Name <> 'packages.json')) then begin
          Log('Unexpected entry in ' + Dir + ': ' + FindRec.Name);
          Result := False;
        end else
          Files := Files + 1;
      until not FindNext(FindRec);
    finally
      FindClose(FindRec);
    end;
  end;
  Result := Result and (Files = 3) and
    HashMatches(Dir + '\oteryn-client.exe', ClientExeSha256) and
    HashMatches(Dir + '\client.env', ClientEnvSha256) and
    HashMatches(Dir + '\packages.json', PackagesSha256);
  if not Result then
    Log('Release directory does not match the payload: ' + Dir);
end;

{ The copy of this release to verify for reuse: the one current.txt names, else the newest. }
function ReuseCandidate(const Releases, Previous: String): String;
var
  Names: TArrayOfString;
  I, CopyNumber, Newest: Integer;
begin
  Result := '';
  if (Previous <> '') and IsCopyOfRelease(Previous, CopyNumber) then begin
    Result := Previous;
    Exit;
  end;
  Newest := -1;
  Names := ListDirectories(Releases);
  for I := 0 to GetArrayLength(Names) - 1 do
    if IsCopyOfRelease(Names[I], CopyNumber) and (CopyNumber > Newest) then begin
      Newest := CopyNumber;
      Result := Names[I];
    end;
end;

{ <release_id> for a first copy, otherwise <release_id>~<n> one above every existing suffix. }
function NewDirectoryName(const Releases: String): String;
var
  Names: TArrayOfString;
  I, CopyNumber, Highest: Integer;
begin
  Highest := -1;
  Names := ListDirectories(Releases);
  for I := 0 to GetArrayLength(Names) - 1 do
    if IsCopyOfRelease(Names[I], CopyNumber) and (CopyNumber > Highest) then
      Highest := CopyNumber;
  if Highest < 0 then
    Result := ReleaseId
  else
    Result := ReleaseId + '~' + IntToStr(Highest + 1);
end;

{ Flushes an existing file's data to disk. }
function FlushFile(const FileName: String): Boolean;
var
  Handle: THandle;
begin
  Handle := CreateFileW(FileName, GENERIC_WRITE, 0, 0, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, 0);
  Result := Handle <> INVALID_HANDLE_VALUE;
  if Result then begin
    Result := FlushFileBuffers(Handle);
    CloseHandle(Handle);
  end;
end;

{ Writes the payload into Staging, flushed and verified. Returns '' or an error. }
function StageRelease(const Staging: String): String;
var
  Names: TArrayOfString;
  I: Integer;
begin
  Result := '';
  if not CreateDir(Staging) then begin
    Result := 'cannot create ' + Staging;
    Exit;
  end;
  SetArrayLength(Names, 3);
  Names[0] := 'oteryn-client.exe';
  Names[1] := 'client.env';
  Names[2] := 'packages.json';
  for I := 0 to GetArrayLength(Names) - 1 do begin
    ExtractTemporaryFile(Names[I]);
    if not CopyFile(ExpandConstant('{tmp}\') + Names[I], Staging + '\' + Names[I], True) or
       not FlushFile(Staging + '\' + Names[I]) then begin
      Result := 'cannot write ' + Staging + '\' + Names[I];
      Exit;
    end;
  end;
  if not ReleaseDirectoryValid(Staging) then
    Result := 'the staged payload does not match the installer';
end;

{ Step 2: the single commit point. }
function Activate(const AppDir, Target: String): String;
var
  NewPointer: String;
  Contents: AnsiString;
  Handle: THandle;
  Written: DWORD;
  Ok: Boolean;
begin
  Result := '';
  NewPointer := AppDir + '\current.txt.new';
  Contents := Target;
  Handle := CreateFileW(NewPointer, GENERIC_WRITE, 0, 0, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, 0);
  if Handle = INVALID_HANDLE_VALUE then begin
    Result := 'cannot create ' + NewPointer + ': ' + SysErrorMessage(DLLGetLastError);
    Exit;
  end;
  Written := 0;
  Ok := WriteFile(Handle, Contents, Length(Contents), Written, 0) and
        (Written = Length(Contents)) and FlushFileBuffers(Handle);
  CloseHandle(Handle);
  if not Ok then begin
    Result := 'cannot write ' + NewPointer;
    Exit;
  end;
  if not MoveFileExW(NewPointer, AppDir + '\current.txt',
                     MOVEFILE_REPLACE_EXISTING or MOVEFILE_WRITE_THROUGH) then
    Result := 'cannot activate ' + Target + ': ' + SysErrorMessage(DLLGetLastError);
end;

{ Step 3, after activation only; failures are harmless and repeated by the next install. }
procedure CleanUp(const Releases, Previous, Target: String);
var
  Names: TArrayOfString;
  I, CopyNumber: Integer;
  SameRelease, Remove: Boolean;
begin
  SameRelease := (Previous <> '') and (ReleaseOf(Previous) = ReleaseId);
  Names := ListDirectories(Releases);
  for I := 0 to GetArrayLength(Names) - 1 do begin
    if SameRelease then
      Remove := IsCopyOfRelease(Names[I], CopyNumber) and (Names[I] <> Target)
    else
      Remove := (Names[I] <> Target) and (Names[I] <> Previous);
    if Remove then begin
      Log('Removing release directory ' + Names[I]);
      if not DelTree(Releases + '\' + Names[I], True, True, True) then
        Log('Could not remove ' + Names[I] + '; the next install retries.');
    end;
  end;
end;

{ Section 2.1 step 1: a verified release directory in TargetRelease. Returns '' or an error;
  current.txt is not touched. }
function PrepareRelease: String;
var
  AppDir, Releases, Previous, Staging, Target: String;
  Names: TArrayOfString;
  I: Integer;
begin
  AppDir := ExpandConstant('{app}');
  Releases := AppDir + '\releases';
  if not ForceDirectories(Releases) then begin
    Result := 'cannot create ' + Releases;
    Exit;
  end;
  Previous := ReadPointer(AppDir, Releases);
  Log('current.txt names: ' + Previous);

  { Leftover staging directories are inert; delete them. }
  Names := ListDirectories(Releases);
  for I := 0 to GetArrayLength(Names) - 1 do
    if Copy(Names[I], 1, 9) = '.staging-' then
      DelTree(Releases + '\' + Names[I], True, True, True);

  Target := ReuseCandidate(Releases, Previous);
  if (Target <> '') and not ReleaseDirectoryValid(Releases + '\' + Target) then
    Target := '';
  if Target <> '' then
    Log('Reusing verified release directory ' + Target)
  else begin
    Staging := Releases + '\.staging-' + ReleaseId;
    if DirExists(Staging) then begin
      Result := 'cannot remove the leftover ' + Staging;
      Exit;
    end;
    Result := StageRelease(Staging);
    if Result <> '' then
      Exit;
    Target := NewDirectoryName(Releases);
    if not RenameFile(Staging, Releases + '\' + Target) then begin
      Result := 'cannot rename ' + Staging + ' to ' + Target;
      Exit;
    end;
    Log('Staged release directory ' + Target);
  end;
  PreviousRelease := Previous;
  TargetRelease := Target;
end;

{ Section 2.1 steps 2-3, after Setup installed its tracked files. Returns '' or an error. }
function ActivateRelease: String;
var
  AppDir: String;
begin
  AppDir := ExpandConstant('{app}');
  Result := Activate(AppDir, TargetRelease);
  if Result <> '' then
    Exit;
  Log('Activated ' + TargetRelease);
  CleanUp(AppDir + '\releases', PreviousRelease, TargetRelease);
end;

{ --- Setup and uninstall events ----------------------------------------------------------- }

function InitializeSetup: Boolean;
begin
  Result := ExpectationMet('/EXPECTRELEASE=', ReleaseId, 'release') and
            ExpectationMet('/EXPECTCHANNEL=', Channel, 'channel') and
            ExpectationMet('/EXPECTVERSION=', ClientVersion, 'version') and
            TakeSetupMutex;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  if MutexHeld(ClientMutexName) then begin
    Result := 'Oteryn is running. Close every Oteryn window, then run the installer again.';
    Log(Result);
    Exit;
  end;
  try
    Result := PrepareRelease;
  except
    Result := GetExceptionMessage;
  end;
  if Result <> '' then begin
    Result := 'Oteryn could not be installed: ' + Result + '. The previously installed release is unchanged.';
    Log(Result);
  end;
end;

{ Activation waits for ssPostInstall so that a failure while Setup installs its tracked files
  leaves current.txt unchanged. A failed activation also leaves it unchanged; Setup then exits
  with ACTIVATION_FAILED_EXIT_CODE. }
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then begin
    try
      ActivationError := ActivateRelease;
    except
      ActivationError := GetExceptionMessage;
    end;
    if ActivationError <> '' then
      Fail('Oteryn could not be activated: ' + ActivationError + '. The previously installed release is unchanged.');
  end;
  if (CurStep = ssDone) and (ActivationError = '') and HasFlag('/RELAUNCH') then
    if not Exec(ExpandConstant('{app}\oteryn-launcher.exe'), '--after-setup', '', SW_SHOWNORMAL,
                ewNoWait, ResultCode) then
      Log('Could not start the launcher: ' + SysErrorMessage(ResultCode));
end;

function GetCustomSetupExitCode: Integer;
begin
  if ActivationError <> '' then
    Result := ACTIVATION_FAILED_EXIT_CODE
  else
    Result := 0;
end;

function InitializeUninstall: Boolean;
begin
  Result := TakeSetupMutex;
  if Result and MutexHeld(ClientMutexName) then begin
    Fail('Oteryn is running. Close every Oteryn window, then uninstall again.');
    Result := False;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: String;
begin
  if CurUninstallStep = usPostUninstall then begin
    { The release directories and current.txt were created by [Code], not logged by Setup. }
    AppDir := ExpandConstant('{app}');
    DelTree(AppDir + '\releases', True, True, True);
    DeleteFile(AppDir + '\current.txt');
    DeleteFile(AppDir + '\current.txt.new');
    RemoveDir(AppDir);
  end;
end;
