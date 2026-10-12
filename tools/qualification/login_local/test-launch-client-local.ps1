# Portable configuration tests. No client process, browser, network or account mutation.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$launcher = Join-Path $PSScriptRoot 'launch-client-local.ps1'
$testDirectory = Join-Path ([IO.Path]::GetTempPath()) ('oteryn-login-launcher-' + [Guid]::NewGuid())
[IO.Directory]::CreateDirectory($testDirectory) | Out-Null
$executable = Join-Path $testDirectory 'oteryn-client.exe'
$environmentFile = Join-Path $testDirectory 'client.env'
$certificate = Join-Path $testDirectory 'login-local-gameplay.crt'
[IO.File]::WriteAllText($executable, '')
[IO.File]::WriteAllText($certificate, 'fixture; actual client validates certificate contents')
$base = @(
    'OTERYN_PLATFORM_URL=https://platform.example.test',
    'OTERYN_GATEWAY_URL=https://gateway.example.test',
    'OTERYN_OAUTH_CLIENT_ID=public-client',
    'OTERYN_DEV_ROOT=/nas/unavailable/certificate.crt',
    'OTERYN_WORLD=ignored-world',
    'OTERYN_CHARACTER_ID=ignored-character',
    '# browser sign-in credentials must not reach output'
)
$checks = 0
function Check([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
    $script:checks++
}
function Refuses([string[]]$Lines) {
    [IO.File]::WriteAllLines($environmentFile, $Lines)
    $refused = $false
    try { & $launcher -ClientPath $executable -ClientEnv $environmentFile -ValidateOnly | Out-Null }
    catch { $refused = $true }
    Check $refused 'Invalid configuration must refuse launch.'
}

try {
    [IO.File]::WriteAllLines($environmentFile, $base)
    $parentBefore = [Environment]::GetEnvironmentVariable('OTERYN_PLATFORM_URL')
    $result = & $launcher -ClientPath $executable -ClientEnv $environmentFile -ValidateOnly
    Check ($result.ClientPath -eq $executable) 'Wrong executable.'
    Check ($result.DevRoot -eq $certificate) 'Copied sibling certificate not selected.'
    Check ($result.AutomaticLogin -eq $false) 'Launcher must not automatically sign in.'
    Check (($result.EnvironmentKeys -join ',') -eq 'OTERYN_PLATFORM_URL,OTERYN_GATEWAY_URL,OTERYN_OAUTH_CLIENT_ID,OTERYN_DEV_ROOT') 'Unexpected child configuration keys.'
    Check ([Environment]::GetEnvironmentVariable('OTERYN_PLATFORM_URL') -eq $parentBefore) 'Parent environment changed.'

    foreach ($origin in @('http://localhost:18564', 'http://127.0.0.1:18564',
        'http://[::1]:18564', 'http://[0000:0000:0000:0000:0000:0000:0000:0001]:18564')) {
        [IO.File]::WriteAllLines($environmentFile, @("OTERYN_PLATFORM_URL=$origin") + $base[1..6])
        $result = & $launcher -ClientPath $executable -ClientEnv $environmentFile -ValidateOnly
        Check (-not $result.AutomaticLogin) 'Loopback origin refused.'
    }
    foreach ($origin in @('http://192.168.1.2', 'https://user:password@example.test',
        'https://example.test/path', 'https://example.test/?token=secret',
        'https://example.test/#fragment', 'file:///tmp', 'not-a-url')) {
        Refuses (@("OTERYN_PLATFORM_URL=$origin") + $base[1..6])
    }
    Refuses ($base + 'OTERYN_PLATFORM_URL=https://duplicate.test')
    Refuses ($base + 'ACCESS_TOKEN=must-not-be-injected')
    Refuses ($base + 'Write-Output unsafe')
    Refuses ($base | Where-Object { -not $_.StartsWith('OTERYN_OAUTH_CLIENT_ID=') })
    Refuses (@('OTERYN_OAUTH_CLIENT_ID=') + ($base | Where-Object { -not $_.StartsWith('OTERYN_OAUTH_CLIENT_ID=') }))

    [IO.File]::WriteAllLines($environmentFile, $base)
    $refused = $false
    try { & $launcher -ClientPath $executable -ClientEnv $environmentFile -DevRoot (Join-Path $testDirectory 'missing.crt') -ValidateOnly | Out-Null }
    catch { $refused = $true }
    Check $refused 'Missing explicit certificate must refuse launch.'

    Write-Output "PASS launcher configuration: $checks checks; no process/network/account action."
} finally {
    # Only this test's unique fixture directory is removed, never owner configuration.
    [IO.Directory]::Delete($testDirectory, $true)
}
