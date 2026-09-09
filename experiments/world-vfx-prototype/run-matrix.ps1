param(
    [int]$Repetitions = 3,
    [int]$WarmupFrames = 120,
    [int]$SampleFrames = 600
)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $root '..\..')).Path
$evidence = Join-Path $root 'evidence\molehill-20260909-world-vfx'
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$rawPath = Join-Path $evidence 'raw.jsonl'
$familyPath = Join-Path $evidence 'family-smoke.jsonl'
$prewarmPath = Join-Path $evidence 'cold-prewarm.jsonl'
$summaryPath = Join-Path $evidence 'summary.json'
$hardwarePath = Join-Path $evidence 'hardware.json'
Remove-Item $rawPath,$familyPath,$prewarmPath,$summaryPath -Force -ErrorAction SilentlyContinue

Push-Location $root
try {
    cargo +1.95.0 test --locked
    cargo +1.95.0 fmt --all --check
    cargo +1.95.0 clippy --locked --all-targets -- -D warnings
    cargo +1.95.0 build --locked --release
} finally {
    Pop-Location
}

$exe = Join-Path $root 'target\release\oteryn-world-vfx-prototype.exe'
if (-not (Test-Path $exe)) { throw "prototype executable missing: $exe" }
$census = Join-Path $repo 'docs\contracts\OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json'
$commit = (git -C $repo rev-parse HEAD).Trim()

function Invoke-Prototype {
    param(
        [string]$Scenario,
        [int]$Density,
        [string]$Layout,
        [string]$Family,
        [string]$Prewarm,
        [int]$Warmup,
        [int]$Frames,
        [int]$Repeat
    )
    $stdout = Join-Path $env:TEMP ("oteryn-world-vfx-{0}.out" -f ([guid]::NewGuid()))
    $stderr = Join-Path $env:TEMP ("oteryn-world-vfx-{0}.err" -f ([guid]::NewGuid()))
    $arguments = @(
        '--scenario', $Scenario,
        '--density', "$Density",
        '--layout', $Layout,
        '--family', $Family,
        '--prewarm', $Prewarm,
        '--warmup', "$Warmup",
        '--frames', "$Frames",
        '--width', '1600',
        '--height', '900',
        '--census', $census
    )
    $process = Start-Process -FilePath $exe -ArgumentList $arguments -PassThru -NoNewWindow `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    [int64]$peak = 0
    while (-not $process.HasExited) {
        $process.Refresh()
        if ($process.PeakWorkingSet64 -gt $peak) { $peak = $process.PeakWorkingSet64 }
        Start-Sleep -Milliseconds 10
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        $errorText = Get-Content $stderr -Raw -ErrorAction SilentlyContinue
        throw "prototype failed ($Scenario/$Density/$Layout/$Family/$Prewarm): $errorText"
    }
    $line = Get-Content $stdout | Where-Object { $_.Trim() } | Select-Object -Last 1
    if (-not $line) { throw "prototype emitted no JSON result" }
    $record = $line | ConvertFrom-Json
    $record | Add-Member -NotePropertyName host -NotePropertyValue ([pscustomobject]@{
        machine = 'Molehill-PC'
        repeat = $Repeat
        peak_working_set_bytes = $peak
        commit_sha = $commit
    })
    Remove-Item $stdout,$stderr -Force -ErrorAction SilentlyContinue
    return ($record | ConvertTo-Json -Depth 12 -Compress)
}

foreach ($scenario in @('basic','normal','stress')) {
    foreach ($density in @(32,64,128)) {
        foreach ($layout in @('atlas','array')) {
            for ($repeat = 1; $repeat -le $Repetitions; $repeat++) {
                $json = Invoke-Prototype -Scenario $scenario -Density $density -Layout $layout `
                    -Family 'enhanced' -Prewarm 'none' -Warmup $WarmupFrames `
                    -Frames $SampleFrames -Repeat $repeat
                Add-Content -Path $rawPath -Value $json -Encoding utf8
            }
        }
    }
}

foreach ($family in @('classic','enhanced','hd')) {
    $json = Invoke-Prototype -Scenario 'normal' -Density 64 -Layout 'array' -Family $family `
        -Prewarm 'none' -Warmup 60 -Frames 360 -Repeat 1
    Add-Content -Path $familyPath -Value $json -Encoding utf8
}

foreach ($prewarm in @('none','critical')) {
    $json = Invoke-Prototype -Scenario 'normal' -Density 64 -Layout 'array' -Family 'enhanced' `
        -Prewarm $prewarm -Warmup 60 -Frames 360 -Repeat 1
    Add-Content -Path $prewarmPath -Value $json -Encoding utf8
}

$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 Name,Manufacturer,NumberOfCores,NumberOfLogicalProcessors
$gpus = Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,AdapterRAM
$os = Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber
$power = (powercfg /getactivescheme | Out-String).Trim()
$hardware = [pscustomobject]@{
    report = 'oteryn-world-vfx-prototype-hardware-v1'
    machine = 'Molehill-PC'
    commit_sha = $commit
    cpu = $cpu
    gpus = $gpus
    os = $os
    active_power_scheme = $power
    vram_counter = [pscustomobject]@{
        status = 'UNAVAILABLE_TRUSTWORTHY'
        reason = 'Win32_VideoController.AdapterRAM is retained only as a raw host field and is not accepted as per-process VRAM evidence'
    }
}
$hardware | ConvertTo-Json -Depth 8 | Set-Content -Path $hardwarePath -Encoding utf8

python (Join-Path $root 'analyze-results.py') $rawPath $familyPath $prewarmPath $summaryPath
Write-Output "WORLD_VFX_EVIDENCE=$evidence"
Write-Output "WORLD_VFX_SUMMARY=$summaryPath"
