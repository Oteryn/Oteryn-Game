param(
    [int]$Repetitions = 3,
    [int]$Warmup = 120,
    [int]$Frames = 600,
    [string]$EvidenceDir = "evidence/molehill-final-20260909",
    [string]$ScenarioFilter = "",
    [string]$ModeFilter = "",
    [string]$DensityFilter = "",
    [string]$Family = "enhanced",
    [string]$AtlasOrigin = "",
    [string]$AtlasSlice = ""
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw "unit tests failed" }
cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { throw "format check failed" }
cargo clippy --locked --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "strict clippy failed" }
cargo build --locked --release
if ($LASTEXITCODE -ne 0) { throw "release build failed" }

$exe = Join-Path $PSScriptRoot "target/release/oteryn-world-vfx-prototype.exe"
$evidence = Join-Path $PSScriptRoot $EvidenceDir
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$resolvedAtlasSlice = ""
$atlasEvidence = $null
if ($AtlasOrigin) {
    $resolvedAtlasSlice = Join-Path $PSScriptRoot "target/atlas-replay/slice.json"
    $atlasManifest = Join-Path $evidence "atlas-slice-manifest.json"
    python (Join-Path $PSScriptRoot "tools/prepare-atlas-slice.py") `
        --origin $AtlasOrigin --output $resolvedAtlasSlice --manifest-output $atlasManifest
    if ($LASTEXITCODE -ne 0) { throw "Atlas slice preparation failed" }
    $atlasEvidence = Get-Content $atlasManifest -Raw | ConvertFrom-Json
} elseif ($AtlasSlice) {
    $resolvedAtlasSlice = (Resolve-Path $AtlasSlice).Path
}
$raw = Join-Path $evidence "raw.jsonl"
$familyRaw = Join-Path $evidence "family-smoke.jsonl"
$stderrLog = Join-Path $evidence "stderr.log"
Set-Content -Path $raw -Value "" -NoNewline
Set-Content -Path $familyRaw -Value "" -NoNewline
Set-Content -Path $stderrLog -Value "" -NoNewline
$commit = (git -C (Resolve-Path (Join-Path $PSScriptRoot '..\..')) rev-parse HEAD).Trim()
$scenarios = if ($ScenarioFilter) { @($ScenarioFilter.Split(',')) } else { @('basic','normal','stress') }
$modes = if ($ModeFilter) { @($ModeFilter.Split(',')) } else { @('atlas','array','hybrid') }
$densities = if ($DensityFilter) { @($DensityFilter.Split(',') | ForEach-Object { [int]$_ }) } else { @(32,64,128) }

function Invoke-PrototypeRun {
    param(
        [string]$Scenario,
        [string]$Mode,
        [int]$Density,
        [string]$PresentationFamily,
        [int]$WarmupFrames,
        [int]$MeasuredFrames,
        [int]$Repeat,
        [string]$Destination
    )
    $stdoutPath = Join-Path $evidence ("run-{0}.out" -f ([guid]::NewGuid()))
    $stderrPath = Join-Path $evidence ("run-{0}.err" -f ([guid]::NewGuid()))
    $arguments = @(
        '--scenario', $Scenario,
        '--resource-mode', $Mode,
        '--family', $PresentationFamily,
        '--density', $Density,
        '--warmup', $WarmupFrames,
        '--frames', $MeasuredFrames
    )
    if ($resolvedAtlasSlice) {
        $arguments += @('--atlas-slice', $resolvedAtlasSlice)
    }
    $process = Start-Process -FilePath $exe -ArgumentList $arguments -PassThru `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    [int64]$peakWorkingSet = 0
    while (-not $process.HasExited) {
        try {
            $sample = Get-Process -Id $process.Id -ErrorAction Stop
            if ($sample.PeakWorkingSet64 -gt $peakWorkingSet) {
                $peakWorkingSet = $sample.PeakWorkingSet64
            }
        } catch {}
        Start-Sleep -Milliseconds 10
        $process.Refresh()
    }
    $process.WaitForExit()
    $jsonLine = Get-Content $stdoutPath | Where-Object { $_.TrimStart().StartsWith('{') } | Select-Object -Last 1
    if (-not $jsonLine) {
        Get-Content $stderrPath | Add-Content $stderrLog
        throw "missing terminal JSON: $Scenario/$Mode/$Density/$PresentationFamily"
    }
    $result = $jsonLine | ConvertFrom-Json
    if ($result.schema -ne 'oteryn-world-vfx-prototype-result-v1') {
        throw "unexpected result schema: $Scenario/$Mode/$Density/$PresentationFamily"
    }
    $result | Add-Member -NotePropertyName matrix_rep -NotePropertyValue $Repeat
    $result | Add-Member -NotePropertyName peak_working_set_bytes -NotePropertyValue $peakWorkingSet
    $result | Add-Member -NotePropertyName commit_sha -NotePropertyValue $commit
    $result | Add-Member -NotePropertyName host_machine -NotePropertyValue 'Molehill-PC'
    if ($atlasEvidence) {
        $result | Add-Member -NotePropertyName atlas_slice_evidence -NotePropertyValue $atlasEvidence
    }
    ($result | ConvertTo-Json -Depth 20 -Compress) | Add-Content $Destination
    if (Test-Path $stderrPath) { Get-Content $stderrPath | Add-Content $stderrLog }
    Remove-Item $stdoutPath,$stderrPath -ErrorAction SilentlyContinue
}
$total = $Repetitions * $scenarios.Count * $modes.Count * $densities.Count
$index = 0
foreach ($rep in 1..$Repetitions) {
    foreach ($scenario in $scenarios) {
        foreach ($density in $densities) {
            foreach ($mode in $modes) {
                $index++
                Write-Host ("[{0}/{1}] rep={2} scenario={3} density={4} mode={5} family={6}" -f `
                    $index,$total,$rep,$scenario,$density,$mode,$Family)
                Invoke-PrototypeRun -Scenario $scenario -Mode $mode -Density $density `
                    -PresentationFamily $Family -WarmupFrames $Warmup -MeasuredFrames $Frames `
                    -Repeat $rep -Destination $raw
                Start-Sleep -Milliseconds 200
            }
        }
    }
}

foreach ($presentationFamily in @('classic','enhanced','hd')) {
    Write-Host ("[family-smoke] family={0}" -f $presentationFamily)
    Invoke-PrototypeRun -Scenario 'normal' -Mode 'array' -Density 64 `
        -PresentationFamily $presentationFamily -WarmupFrames 60 -MeasuredFrames 360 `
        -Repeat 1 -Destination $familyRaw
}
Write-Host "matrix-complete=$total raw=$raw family=$familyRaw"
$hardware = [ordered]@{
    report = 'oteryn-world-vfx-prototype-hardware-v1'
    machine = 'Molehill-PC'
    commit_sha = $commit
    cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 Name,Manufacturer,NumberOfCores,NumberOfLogicalProcessors
    gpus = @(Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,AdapterRAM)
    os = Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber
    active_power_scheme = (powercfg /getactivescheme | Out-String).Trim()
    vram_counter = [ordered]@{
        status = 'UNAVAILABLE_TRUSTWORTHY'
        reason = 'No accepted per-process VRAM counter was available to this harness.'
    }
}
$hardware | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $evidence 'hardware.json') -Encoding utf8
python (Join-Path $PSScriptRoot 'analyze-results.py') $raw $familyRaw (Join-Path $evidence 'summary.json')
if ($LASTEXITCODE -ne 0) { throw 'analysis failed' }
Write-Host ("summary={0}" -f (Join-Path $evidence 'summary.json'))