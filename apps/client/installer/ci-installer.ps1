# Builds and exercises the unsigned Windows installer in the rust_windows merge-gate job
# (CLIENT-INSTALLER-0 section 7). Run from the repository root after the release build of
# oteryn-client with OTERYN_RELEASE_ID = $ReleaseId. Every failure throws; nothing is optional.
#
# Two installers are built: A for $ReleaseId and a fixture B (<version>+dev.ci.g000000000000,
# a rebuild of the same commit) so that the distinct-previous-release rules of section 2.1 can be
# checked. Only A is published.
param(
    [Parameter(Mandatory = $true)] [string] $ReleaseId,
    [Parameter(Mandatory = $true)] [string] $ClientVersion,
    [Parameter(Mandatory = $true)] [string] $GameCommit,
    [Parameter(Mandatory = $true)] [string] $Iscc,
    [Parameter(Mandatory = $true)] [string] $WorkDir,
    [Parameter(Mandatory = $true)] [string] $OutputDir
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
Set-StrictMode -Version Latest

$Target = 'x86_64-pc-windows-msvc'
$Channel = 'dev'
$Installer = Join-Path $PSScriptRoot 'oteryn-client.iss'
$Sign = Join-Path $PSScriptRoot 'sign.ps1'
$Release = Join-Path (Get-Location) "target\$Target\release"
$FixtureId = "$ClientVersion+$Channel.ci.g000000000000"
$InstallDir = Join-Path $WorkDir 'install\Programs\Oteryn'
$UserData = Join-Path $WorkDir 'install\Oteryn'
$Logs = Join-Path $WorkDir 'logs'
$Sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$SetupMutexName = "Global\OterynClientSetup-$Sid"
$ClientMutexName = "Global\OterynClient-$Sid"
$UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{9355097F-B319-4FEC-9314-3661537B6A22}_is1'
$script:Run = 0

function Check([bool] $Condition, [string] $Message) {
    if (-not $Condition) { throw "installer check failed: $Message" }
}

function Sha256([string] $Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-Utf8([string] $Path, [string] $Text) {
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}

# The release payload of section 2.1: the client, the dev client.env and an empty packages.json.
function New-Payload([string] $Id, [string] $Commit) {
    $payload = Join-Path $WorkDir "payload\$Id"
    $launcherDir = Join-Path $WorkDir "launcher\$Id"
    New-Item -ItemType Directory -Force -Path $payload, $launcherDir | Out-Null
    Copy-Item -LiteralPath (Join-Path $Release 'oteryn-client.exe') -Destination $payload
    Copy-Item -LiteralPath (Join-Path $Release 'oteryn-launcher.exe') -Destination $launcherDir
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "client.env.$Channel") -Destination (Join-Path $payload 'client.env')
    $manifest = [ordered]@{
        schema   = 'oteryn.client.packages.v1'
        release  = [ordered]@{
            release_id     = $Id
            client_version = $ClientVersion
            channel        = $Channel
            game_commit    = $Commit
            target         = $Target
        }
        packages = @()
    }
    Write-Utf8 (Join-Path $payload 'packages.json') (($manifest | ConvertTo-Json -Depth 4 -Compress) + "`n")
    Check ((Get-Content -Raw -LiteralPath (Join-Path $payload 'client.env')) -notmatch '(?m)^\s*(OTERYN_CHARACTER_ID|OTERYN_DEV_ROOT)\s*=') 'client.env ships a per-user-only key'
    $exe = [IO.File]::ReadAllBytes((Join-Path $payload 'oteryn-client.exe'))
    $needle = [Text.Encoding]::ASCII.GetBytes("oteryn-client/$Id")
    $text = [Text.Encoding]::Latin1.GetString($exe)
    Check ($text.Contains([Text.Encoding]::Latin1.GetString($needle))) "oteryn-client.exe does not embed oteryn-client/$Id"
    $payload
}

function New-Installer([string] $Id, [string] $Payload) {
    $launcher = Join-Path $WorkDir "launcher\$Id\oteryn-launcher.exe"
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    & $Iscc /Qp "/DReleaseId=$Id" "/DClientVersion=$ClientVersion" "/DChannel=$Channel" `
        "/DPayloadDir=$Payload" "/DLauncherPath=$launcher" "/DOutputDir=$OutputDir" `
        "/DClientExeSha256=$(Sha256 (Join-Path $Payload 'oteryn-client.exe'))" `
        "/DClientEnvSha256=$(Sha256 (Join-Path $Payload 'client.env'))" `
        "/DPackagesSha256=$(Sha256 (Join-Path $Payload 'packages.json'))" $Installer | Out-Host
    Check ($LASTEXITCODE -eq 0) "ISCC failed for $Id with exit code $LASTEXITCODE"
    $setup = Join-Path $OutputDir "oteryn-client-$Id-x86_64-setup.exe"
    Check (Test-Path -LiteralPath $setup -PathType Leaf) "ISCC produced no $setup"
    & pwsh -NoProfile -File $Sign $setup | Out-Host
    Check ($LASTEXITCODE -eq 0) "sign.ps1 failed for $setup"
    $setup
}

# Start-Process -Wait also waits for descendants, so a client that /RELAUNCH starts would block
# it forever. Wait for the setup process alone, with a bound.
function Wait-Run([Diagnostics.Process] $Process, [string] $Name) {
    $null = $Process.Handle
    if (-not $Process.WaitForExit(300000)) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
        throw "$Name timed out"
    }
    $Process
}

function Invoke-Setup([string] $Setup, [string[]] $Extra = @()) {
    Check (Test-Path -LiteralPath $Setup -PathType Leaf) "no setup executable at '$Setup'"
    $script:Run++
    $log = Join-Path $Logs "setup-$script:Run.log"
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/DIR=`"$InstallDir`"", "/LOG=`"$log`"") + $Extra
    $process = Wait-Run (Start-Process -FilePath $Setup -ArgumentList $arguments -PassThru) "setup run $script:Run"
    Write-Host "setup run $script:Run ($([IO.Path]::GetFileName($Setup)) $Extra) exited $($process.ExitCode)"
    $process.ExitCode
}

function Invoke-Uninstall {
    $script:Run++
    $log = Join-Path $Logs "uninstall-$script:Run.log"
    $uninstaller = Join-Path $InstallDir 'unins000.exe'
    Check (Test-Path -LiteralPath $uninstaller -PathType Leaf) 'unins000.exe is missing'
    $process = Wait-Run (Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$log`"") -PassThru) "uninstall run $script:Run"
    Write-Host "uninstall run $script:Run exited $($process.ExitCode)"
    $process.ExitCode
}

function Current {
    $path = Join-Path $InstallDir 'current.txt'
    Check (Test-Path -LiteralPath $path -PathType Leaf) 'current.txt is missing'
    [IO.File]::ReadAllText($path)
}

function Release-Dirs {
    @(Get-ChildItem -LiteralPath (Join-Path $InstallDir 'releases') -Directory -Force | ForEach-Object Name | Sort-Object)
}

function Assert-Releases([string[]] $Expected) {
    $actual = (Release-Dirs) -join ','
    $wanted = (@($Expected | Sort-Object)) -join ','
    Check ($actual -ceq $wanted) "release directories are [$actual], expected [$wanted]"
}

# Section 2.1 layout, and the active release runs.
function Assert-Active([string] $Directory) {
    Check ((Current) -ceq $Directory) "current.txt is '$(Current)', expected '$Directory'"
    Check (Test-Path -LiteralPath (Join-Path $InstallDir 'oteryn-launcher.exe') -PathType Leaf) 'oteryn-launcher.exe is missing'
    $fontNotice = Join-Path $InstallDir 'THIRD-PARTY-FONTS.txt'
    Check (Test-Path -LiteralPath $fontNotice -PathType Leaf) 'embedded font notices are missing'
    Check ((Sha256 $fontNotice) -ceq (Sha256 (Join-Path $PSScriptRoot '..\THIRD-PARTY-FONTS.txt'))) 'installed font notices differ from the source'
    Check (-not (Test-Path -LiteralPath (Join-Path $InstallDir 'current.txt.new'))) 'current.txt.new was left behind'
    $releaseDir = Join-Path $InstallDir "releases\$Directory"
    $files = @(Get-ChildItem -LiteralPath $releaseDir -Force | ForEach-Object Name | Sort-Object) -join ','
    Check ($files -ceq 'client.env,oteryn-client.exe,packages.json') "release $Directory holds [$files]"
    $manifest = Get-Content -Raw -LiteralPath (Join-Path $releaseDir 'packages.json') | ConvertFrom-Json
    Check ($manifest.release.release_id -ceq ($Directory -replace '~\d+$', '')) "packages.json of $Directory names $($manifest.release.release_id)"
    & (Join-Path $releaseDir 'oteryn-client.exe') --smoke
    Check ($LASTEXITCODE -eq 0) "installed $Directory --smoke exited $LASTEXITCODE"
}

function Hold-Mutex([string] $Name) {
    $mutex = [Threading.Mutex]::new($false, $Name)
    $mutex
}

function Mutex-Exists([string] $Name) {
    $mutex = $null
    if ([Threading.Mutex]::TryOpenExisting($Name, [ref] $mutex)) {
        $mutex.Dispose()
        return $true
    }
    $false
}

function Wait-Until([scriptblock] $Condition, [int] $Seconds, [string] $What) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    while (-not (& $Condition)) {
        Check ([DateTime]::UtcNow -lt $deadline) "timed out waiting for $What"
        Start-Sleep -Milliseconds 250
    }
}

New-Item -ItemType Directory -Force -Path $WorkDir, $Logs, $UserData | Out-Null
Check (-not (Test-Path -LiteralPath $InstallDir)) "$InstallDir already exists"
$sentinel = Join-Path $UserData 'client.env'
Write-Utf8 $sentinel "# per-user data that uninstall must keep`n"
$sentinelHash = Sha256 $sentinel

# Installer A (published) from the release build of this commit.
$payloadA = New-Payload $ReleaseId $GameCommit
$setupA = New-Installer $ReleaseId $payloadA

# Fixture installer B: the same commit rebuilt with another release id.
$env:OTERYN_RELEASE_ID = $FixtureId
& cargo +1.94.0 build --locked --release -p oteryn-client --target $Target
Check ($LASTEXITCODE -eq 0) "fixture build failed with exit code $LASTEXITCODE"
$payloadB = New-Payload $FixtureId $GameCommit
$fixtureOutput = Join-Path $WorkDir 'fixture'
$publishOutput = $OutputDir
$OutputDir = $fixtureOutput
$setupB = New-Installer $FixtureId $payloadB
$OutputDir = $publishOutput
$env:OTERYN_RELEASE_ID = $ReleaseId

Write-Host '--- First install (B)'
Check ((Invoke-Setup $setupB) -eq 0) 'first install failed'
Assert-Active $FixtureId
Assert-Releases @($FixtureId)
Check (Test-Path -LiteralPath $UninstallKey) 'the HKCU uninstall entry is missing'

Write-Host '--- Install A over B: A active, B kept as the previous distinct release'
Check ((Invoke-Setup $setupA) -eq 0) 'install of A failed'
Assert-Active $ReleaseId
Assert-Releases @($ReleaseId, $FixtureId)

Write-Host '--- Same-release reinstall of A: the verified directory is reused, B is kept'
$stamp = (Get-Item -LiteralPath (Join-Path $InstallDir "releases\$ReleaseId")).CreationTimeUtc
Check ((Invoke-Setup $setupA) -eq 0) 'reinstall of A failed'
Assert-Active $ReleaseId
Assert-Releases @($ReleaseId, $FixtureId)
Check ((Get-Item -LiteralPath (Join-Path $InstallDir "releases\$ReleaseId")).CreationTimeUtc -eq $stamp) 'the release directory was rewritten instead of reused'

Write-Host '--- Repair: a corrupted A is replaced by A~1, activated, then the broken A is removed'
Add-Content -LiteralPath (Join-Path $InstallDir "releases\$ReleaseId\client.env") -Value 'OTERYN_WORLD=tampered'
Check ((Invoke-Setup $setupA) -eq 0) 'repair install failed'
Assert-Active "$ReleaseId~1"
Assert-Releases @("$ReleaseId~1", $FixtureId)
$repairLog = Get-Content -Raw -LiteralPath (Join-Path $Logs "setup-$script:Run.log")
$activated = $repairLog.IndexOf("Activated $ReleaseId~1")
$removed = $repairLog.IndexOf("Removing release directory $ReleaseId`r")
Check ($activated -ge 0 -and $removed -gt $activated) 'the broken directory was not removed after activation'

Write-Host '--- Interrupted install: a stuck staging directory fails the install and leaves A~1 active'
Remove-Item -LiteralPath (Join-Path $InstallDir "releases\$FixtureId") -Recurse -Force
$staging = Join-Path $InstallDir "releases\.staging-$FixtureId"
New-Item -ItemType Directory -Path $staging | Out-Null
$lockPath = Join-Path $staging 'oteryn-client.exe'
$lock = [IO.File]::Open($lockPath, 'Create', 'ReadWrite', 'None')
try {
    $code = Invoke-Setup $setupB
    Check ($code -eq 7) "install over a locked staging directory exited $code, expected 7"
    Assert-Active "$ReleaseId~1"
    Check (-not (Test-Path -LiteralPath (Join-Path $InstallDir "releases\$FixtureId"))) 'a partial release directory was created'
} finally {
    $lock.Dispose()
}
Write-Host '--- Recovery: a leftover staging directory is discarded by the next install'
Check ((Invoke-Setup $setupB) -eq 0) 'install after an interrupted install failed'
Assert-Active $FixtureId
Assert-Releases @($FixtureId, "$ReleaseId~1")

Write-Host '--- Embedded identity must match the expected pointer values'
$code = Invoke-Setup $setupA @("/EXPECTRELEASE=$FixtureId")
Check ($code -ne 0) 'an installer for another release_id was accepted'
$code = Invoke-Setup $setupA @("/EXPECTCHANNEL=preproduction")
Check ($code -ne 0) 'an installer for another channel was accepted'
$code = Invoke-Setup $setupA @("/EXPECTVERSION=0.0.0")
Check ($code -ne 0) 'an installer for another version was accepted'
Assert-Active $FixtureId
Check ((Invoke-Setup $setupA @("/EXPECTRELEASE=$ReleaseId", "/EXPECTCHANNEL=$Channel", "/EXPECTVERSION=$ClientVersion")) -eq 0) 'a matching expected identity was refused'
# The newest verified copy of A is reused and B stays as the previous distinct release.
$activeA = "$ReleaseId~1"
Assert-Active $activeA
Assert-Releases @($activeA, $FixtureId)

Write-Host '--- Concurrent installer: a second install is refused while the first holds the transaction mutex'
$before = Current
$first = Start-Process -FilePath $setupB -ArgumentList @("/DIR=`"$InstallDir`"", "/LOG=`"$(Join-Path $Logs 'concurrent-first.log')`"") -PassThru
try {
    Wait-Until { Mutex-Exists $SetupMutexName } 60 'the first installer to take the transaction mutex'
    $code = Invoke-Setup $setupB
    Check ($code -ne 0) 'a second installer ran while the first held the transaction mutex'
    Check ((Current) -ceq $before) 'the second installer changed current.txt'
} finally {
    & taskkill /PID $first.Id /T /F | Out-Host
}
Wait-Until { -not (Mutex-Exists $SetupMutexName) } 60 'the first installer to exit'
Assert-Active $activeA

Write-Host '--- Transaction mutex held: uninstall and client start are refused'
$held = Hold-Mutex $SetupMutexName
try {
    $code = Invoke-Uninstall
    Check ($code -ne 0) 'uninstall ran while setup held the transaction mutex'
    Check (Test-Path -LiteralPath (Join-Path $InstallDir 'unins000.exe')) 'the refused uninstall removed files'
    & (Join-Path $InstallDir "releases\$activeA\oteryn-client.exe") --smoke
    Check ($LASTEXITCODE -eq 3) "the client started while setup held the transaction mutex (exit $LASTEXITCODE)"
} finally {
    $held.Dispose()
}
Assert-Active $activeA

Write-Host '--- Client mutex held: install and uninstall are refused'
$held = Hold-Mutex $ClientMutexName
try {
    $code = Invoke-Setup $setupB
    Check ($code -eq 7) "install while the client runs exited $code, expected 7"
    Check ((Current) -ceq $activeA) 'the refused install changed current.txt'
    $code = Invoke-Uninstall
    Check ($code -ne 0) 'uninstall ran while the client held its mutex'
    Wait-Until { -not (Mutex-Exists $SetupMutexName) } 60 'the refused uninstaller to exit'
} finally {
    $held.Dispose()
}
Assert-Active $activeA

Write-Host '--- Launcher --after-setup starts the client only after the transaction mutex is released'
$held = Hold-Mutex $SetupMutexName
try {
    $launcher = Start-Process -FilePath (Join-Path $InstallDir 'oteryn-launcher.exe') -ArgumentList @('--after-setup', '--smoke') -PassThru
    Start-Sleep -Seconds 3
    Check (-not $launcher.HasExited) 'the launcher did not wait for the transaction mutex'
    Check (-not (Mutex-Exists $ClientMutexName)) 'the client started while setup held the transaction mutex'
} finally {
    $held.Dispose()
}
Check ($launcher.WaitForExit(60000)) 'the launcher did not exit after the transaction mutex was released'
Check ($launcher.ExitCode -eq 0) "the launcher exited $($launcher.ExitCode)"
Wait-Until { -not (Mutex-Exists $ClientMutexName) } 120 'the launched smoke client to exit'

Write-Host '--- /RELAUNCH starts the active client through the launcher after setup exits'
Check ((Invoke-Setup $setupA @('/RELAUNCH')) -eq 0) 'install with /RELAUNCH failed'
Wait-Until { Mutex-Exists $ClientMutexName } 60 'the relaunched client'
$clientExe = Join-Path $InstallDir "releases\$activeA\oteryn-client.exe"
Get-Process -Name 'oteryn-client' -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -eq $clientExe } | Stop-Process -Force
Wait-Until { -not (Mutex-Exists $ClientMutexName) } 60 'the relaunched client to stop'
Assert-Active $activeA

Write-Host '--- Uninstall removes the install directory and keeps per-user data'
Check ((Invoke-Uninstall) -eq 0) 'uninstall failed'
Wait-Until { -not (Test-Path -LiteralPath $InstallDir) } 120 'the install directory to be removed'
Check (-not (Test-Path -LiteralPath $UninstallKey)) 'the HKCU uninstall entry was left behind'
Check ((Sha256 $sentinel) -eq $sentinelHash) 'uninstall changed per-user data'

Write-Host '--- SHA256SUMS'
$published = Join-Path $OutputDir "oteryn-client-$ReleaseId-x86_64-setup.exe"
Copy-Item -LiteralPath (Join-Path $payloadA 'packages.json') -Destination $OutputDir
$sums = @($published, (Join-Path $OutputDir 'packages.json')) |
    ForEach-Object { "$(Sha256 $_)  $([IO.Path]::GetFileName($_))" }
Write-Utf8 (Join-Path $OutputDir 'SHA256SUMS') (($sums -join "`n") + "`n")
Get-Content -LiteralPath (Join-Path $OutputDir 'SHA256SUMS') | Out-Host
Write-Host 'installer checks passed'
