param(
    [ValidateRange(1, 20)]
    [int]$Repetitions = 5,
    [ValidateRange(1, 10000)]
    [int]$WarmupFrames = 180,
    [ValidateRange(1, 100000)]
    [int]$SampleFrames = 900,
    [string]$HardwareAlias = "Molehill-PC",
    [switch]$SkipBuild,
    [ValidateRange(0, 5000)]
    [int]$InterRunDelayMs = 500,
    [ValidateRange(1, 5)]
    [int]$MaxAttempts = 3
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# Keep both wgpu generations on the same Windows backend and prefer the discrete GPU.
$env:WGPU_BACKEND = "dx12"
$env:WGPU_POWER_PREF = "high"

$Root = $PSScriptRoot
$Manifest = Join-Path $Root "Cargo.toml"
$Results = Join-Path $Root "results"
New-Item -ItemType Directory -Force -Path $Results | Out-Null

$Timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$RawPath = Join-Path $Results "raw-$Timestamp.jsonl"
$FailurePath = Join-Path $Results "failures-$Timestamp.jsonl"
$BuildPath = Join-Path $Results "build-$Timestamp.json"

function Invoke-CargoBuild {
    param(
        [Parameter(Mandatory = $true)][string]$Package,
        [Parameter(Mandatory = $true)][string]$TargetDir
    )

    if (Test-Path $TargetDir) {
        Remove-Item -Recurse -Force $TargetDir
    }
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo +1.95.0 build --manifest-path $Manifest --release -p $Package --target-dir $TargetDir
    $exit = $LASTEXITCODE
    $watch.Stop()
    if ($exit -ne 0) {
        throw "cargo build failed for $Package with exit code $exit"
    }
    return $watch.Elapsed.TotalSeconds
}

function Get-MachineEvidence {
    $os = Get-CimInstance Win32_OperatingSystem
    $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
    $gpus = @(Get-CimInstance Win32_VideoController | ForEach-Object {
        [ordered]@{
            name = $_.Name
            driver_version = $_.DriverVersion
        }
    })
    $power = (& powercfg /GETACTIVESCHEME 2>&1 | Out-String).Trim()
    return [ordered]@{
        hardware_alias = $HardwareAlias
        os_caption = $os.Caption
        os_version = $os.Version
        os_build = $os.BuildNumber
        cpu = $cpu.Name
        logical_processors = $cpu.NumberOfLogicalProcessors
        gpu = $gpus
        power_scheme = $power
        wgpu_backend = $env:WGPU_BACKEND
        wgpu_power_preference = $env:WGPU_POWER_PREF
    }
}

function Invoke-BenchmarkCell {
    param(
        [Parameter(Mandatory = $true)][string]$Backend,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string]$Scenario,
        [Parameter(Mandatory = $true)][int]$SpritePx,
        [Parameter(Mandatory = $true)][int]$Repetition,
        [Parameter(Mandatory = $true)][int]$Attempt,
        [Parameter(Mandatory = $true)]$Machine
    )

    $stdout = Join-Path $env:TEMP "oteryn-bakeoff-$PID-$Backend-$Scenario-$SpritePx-$Repetition-$Attempt.out"
    $stderr = Join-Path $env:TEMP "oteryn-bakeoff-$PID-$Backend-$Scenario-$SpritePx-$Repetition-$Attempt.err"
    Remove-Item -Force -ErrorAction SilentlyContinue $stdout, $stderr

    $arguments = @(
        "--scenario", $Scenario,
        "--sprite-px", "$SpritePx",
        "--warmup", "$WarmupFrames",
        "--frames", "$SampleFrames"
    )

    $process = Start-Process -FilePath $Executable -ArgumentList $arguments -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr

    [long]$peakWorkingSet = 0
    while (-not $process.HasExited) {
        try {
            $process.Refresh()
            if ($process.WorkingSet64 -gt $peakWorkingSet) {
                $peakWorkingSet = $process.WorkingSet64
            }
        } catch {
            # The process can exit between HasExited and Refresh; keep the last valid sample.
        }
        Start-Sleep -Milliseconds 10
    }
    $process.WaitForExit()
    $process.Refresh()

    $exitCode = $null
    try {
        $exitCode = $process.ExitCode
    } catch {
        $exitCode = $null
    }

    $stdoutLines = if (Test-Path $stdout) { @(Get-Content $stdout) } else { @() }
    $jsonLine = $stdoutLines | Where-Object { $_ -match '^\{.*\}$' } | Select-Object -Last 1
    if (-not $jsonLine) {
        $errorText = if (Test-Path $stderr) { Get-Content -Raw $stderr } else { "" }
        throw "$Backend/$Scenario/$SpritePx repetition $Repetition attempt $Attempt produced no result JSON; exit=${exitCode}; stderr=$errorText"
    }
    if ($null -ne $exitCode -and $exitCode -ne 0) {
        $errorText = if (Test-Path $stderr) { Get-Content -Raw $stderr } else { "" }
        throw "$Backend/$Scenario/$SpritePx repetition $Repetition attempt $Attempt failed with exit ${exitCode}: $errorText"
    }

    $adapterLog = $stdoutLines | Where-Object { $_ -match 'AdapterInfo.*name:' } | Select-Object -Last 1
    $record = $jsonLine | ConvertFrom-Json
    $record | Add-Member -NotePropertyName repetition -NotePropertyValue $Repetition
    $record | Add-Member -NotePropertyName attempt -NotePropertyValue $Attempt
    $record | Add-Member -NotePropertyName peak_working_set_bytes -NotePropertyValue $peakWorkingSet
    $record | Add-Member -NotePropertyName adapter_log -NotePropertyValue $adapterLog
    $record | Add-Member -NotePropertyName hardware -NotePropertyValue $Machine
    $record | Add-Member -NotePropertyName captured_at_utc -NotePropertyValue ((Get-Date).ToUniversalTime().ToString("o"))
    ($record | ConvertTo-Json -Depth 8 -Compress) | Add-Content -Encoding utf8 $RawPath

    Remove-Item -Force -ErrorAction SilentlyContinue $stdout, $stderr
}

$Machine = Get-MachineEvidence
$GitHead = (& git -C $Root rev-parse HEAD | Out-String).Trim()
$LockPath = Join-Path $Root "Cargo.lock"
$LockSha256 = if (Test-Path $LockPath) {
    (Get-FileHash -Algorithm SHA256 $LockPath).Hash.ToLowerInvariant()
} else {
    $null
}

$WgpuTarget = Join-Path $Root "target-physical-wgpu"
$BevyTarget = Join-Path $Root "target-physical-bevy"
$WgpuBuildSeconds = $null
$BevyBuildSeconds = $null
if (-not $SkipBuild) {
    $WgpuBuildSeconds = Invoke-CargoBuild -Package "oteryn-graphics-bakeoff-wgpu" -TargetDir $WgpuTarget
    $BevyBuildSeconds = Invoke-CargoBuild -Package "oteryn-graphics-bakeoff-bevy" -TargetDir $BevyTarget
}

$WgpuExe = Join-Path $WgpuTarget "release\oteryn-graphics-bakeoff-wgpu.exe"
$BevyExe = Join-Path $BevyTarget "release\oteryn-graphics-bakeoff-bevy.exe"
if (-not (Test-Path $WgpuExe) -or -not (Test-Path $BevyExe)) {
    throw "expected release benchmark executables were not produced; omit -SkipBuild for a clean build"
}

$buildEvidence = [ordered]@{
    captured_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    git_head = $GitHead
    cargo_lock_sha256 = $LockSha256
    hardware = $Machine
    rust = (& rustc +1.95.0 --version | Out-String).Trim()
    build_skipped = [bool]$SkipBuild
    wgpu_clean_build_seconds = $WgpuBuildSeconds
    bevy_clean_build_seconds = $BevyBuildSeconds
    wgpu_binary_bytes = (Get-Item $WgpuExe).Length
    bevy_binary_bytes = (Get-Item $BevyExe).Length
    repetitions = $Repetitions
    warmup_frames = $WarmupFrames
    sample_frames = $SampleFrames
    inter_run_delay_ms = $InterRunDelayMs
    max_attempts = $MaxAttempts
}
$buildEvidence | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $BuildPath

$backends = @(
    @{ name = "custom-wgpu"; executable = $WgpuExe },
    @{ name = "bevy"; executable = $BevyExe }
)
$scenarios = @("basic", "normal", "stress")
$densities = @(32, 64, 128)

foreach ($scenario in $scenarios) {
    foreach ($spritePx in $densities) {
        foreach ($repetition in 1..$Repetitions) {
            foreach ($backend in $backends) {
                $success = $false
                for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
                    Write-Host "RUN $($backend.name) scenario=$scenario sprite=$spritePx repetition=$repetition/$Repetitions attempt=$attempt/$MaxAttempts"
                    try {
                        Invoke-BenchmarkCell -Backend $backend.name -Executable $backend.executable `
                            -Scenario $scenario -SpritePx $spritePx -Repetition $repetition `
                            -Attempt $attempt -Machine $Machine
                        $success = $true
                        break
                    } catch {
                        $failure = [ordered]@{
                            captured_at_utc = (Get-Date).ToUniversalTime().ToString("o")
                            backend = $backend.name
                            scenario = $scenario
                            sprite_px = $spritePx
                            repetition = $repetition
                            attempt = $attempt
                            error = $_.Exception.Message
                        }
                        ($failure | ConvertTo-Json -Compress) | Add-Content -Encoding utf8 $FailurePath
                        Write-Warning "transient benchmark failure recorded; retrying if attempts remain"
                        Start-Sleep -Milliseconds ([Math]::Max(1000, $InterRunDelayMs))
                    }
                }
                if (-not $success) {
                    throw "$($backend.name)/$scenario/$spritePx repetition $repetition exhausted $MaxAttempts attempts"
                }
                if ($InterRunDelayMs -gt 0) {
                    Start-Sleep -Milliseconds $InterRunDelayMs
                }
            }
        }
    }
}

Write-Host "RAW_RESULTS=$RawPath"
Write-Host "FAILURES=$FailurePath"
Write-Host "BUILD_RESULTS=$BuildPath"
