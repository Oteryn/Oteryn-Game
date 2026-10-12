# Testing/preproduction only. Reuses the client's browser PKCE flow; no password or grant injection.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ClientPath,
    [Parameter(Mandatory = $true)][string]$ClientEnv,
    [string]$DevRoot,
    [switch]$ValidateOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-InputFile([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Label file does not exist."
    }
    return (Get-Item -LiteralPath $Path).FullName
}

function Assert-Origin([string]$Value, [string]$Label) {
    $uri = $null
    if (-not [Uri]::TryCreate($Value, [UriKind]::Absolute, [ref]$uri)) {
        throw "$Label must be an absolute origin."
    }
    # .NET Framework (Windows PowerShell 5.1) expands IPv6 Uri.Host; modern .NET compresses it.
    $loopback = $uri.Host -in @('localhost', '127.0.0.1', '[::1]', '::1',
        '[0000:0000:0000:0000:0000:0000:0000:0001]')
    if (($uri.Scheme -ne 'https' -and -not ($uri.Scheme -eq 'http' -and $loopback)) -or
        [string]::IsNullOrEmpty($uri.Host) -or $uri.UserInfo -ne '' -or
        $uri.AbsolutePath -ne '/' -or $uri.Query -ne '' -or $uri.Fragment -ne '') {
        throw "$Label requires HTTPS (HTTP is allowed only on loopback), without credentials, path, query or fragment."
    }
}

$executable = Resolve-InputFile $ClientPath 'Client'
if ([IO.Path]::GetExtension($executable) -ne '.exe') {
    throw 'ClientPath must identify the compiled Windows client executable.'
}
$environmentFile = Resolve-InputFile $ClientEnv 'Client environment'
$allowed = @('OTERYN_PLATFORM_URL', 'OTERYN_GATEWAY_URL', 'OTERYN_OAUTH_CLIENT_ID',
    'OTERYN_DEV_ROOT', 'OTERYN_WORLD', 'OTERYN_CHARACTER_ID')
$required = @('OTERYN_PLATFORM_URL', 'OTERYN_GATEWAY_URL', 'OTERYN_OAUTH_CLIENT_ID')
$values = @{}
# Get-Content closes the file before validation can throw. PowerShell 5.1 does
# not reliably dispose a lazy ReadLines enumerator on an early refusal.
foreach ($line in @(Get-Content -LiteralPath $environmentFile -Encoding UTF8)) {
    $entry = $line.Trim()
    # The runbook includes test credentials in a comment. Never evaluate, display or propagate it.
    if ($entry -eq '' -or $entry.StartsWith('#')) { continue }
    if ($entry -notmatch '^([A-Z_]+)=(.*)$') {
        throw 'Invalid client environment entry; expected KEY=value, not executable shell syntax.'
    }
    $name = $Matches[1]
    $value = $Matches[2]
    if ($name -notin $allowed -or $values.ContainsKey($name)) {
        throw 'Unknown or duplicate client environment key.'
    }
    if ($value.Contains([char]0)) { throw 'Invalid client environment value.' }
    $values[$name] = $value
}
foreach ($name in $required) {
    if (-not $values.ContainsKey($name) -or [string]::IsNullOrWhiteSpace($values[$name])) {
        throw "Missing $name in client environment."
    }
}
Assert-Origin $values['OTERYN_PLATFORM_URL'] 'Platform URL'
Assert-Origin $values['OTERYN_GATEWAY_URL'] 'Gateway URL'

# The copied NAS environment may contain a Linux certificate path. Resolve locally, never install trust.
if ([string]::IsNullOrWhiteSpace($DevRoot)) {
    $sibling = Join-Path ([IO.Path]::GetDirectoryName($environmentFile)) 'login-local-gameplay.crt'
    if (Test-Path -LiteralPath $sibling -PathType Leaf) {
        $DevRoot = $sibling
    } elseif ($values.ContainsKey('OTERYN_DEV_ROOT')) {
        $DevRoot = $values['OTERYN_DEV_ROOT']
        if (-not [IO.Path]::IsPathRooted($DevRoot)) {
            $DevRoot = Join-Path ([IO.Path]::GetDirectoryName($environmentFile)) $DevRoot
        }
    } else {
        throw 'Supply the local gameplay certificate using -DevRoot.'
    }
}
$certificate = Resolve-InputFile $DevRoot 'Gameplay certificate'

$start = New-Object System.Diagnostics.ProcessStartInfo
$start.FileName = $executable
$start.WorkingDirectory = [IO.Path]::GetDirectoryName($executable)
$start.UseShellExecute = $false
$start.EnvironmentVariables['OTERYN_PLATFORM_URL'] = $values['OTERYN_PLATFORM_URL']
$start.EnvironmentVariables['OTERYN_GATEWAY_URL'] = $values['OTERYN_GATEWAY_URL']
$start.EnvironmentVariables['OTERYN_OAUTH_CLIENT_ID'] = $values['OTERYN_OAUTH_CLIENT_ID']
$start.EnvironmentVariables['OTERYN_DEV_ROOT'] = $certificate
# The GUI selects from the authenticated owner's projection, never copied/configured character IDs.
$start.EnvironmentVariables.Remove('OTERYN_WORLD')
$start.EnvironmentVariables.Remove('OTERYN_CHARACTER_ID')

if ($ValidateOnly) {
    [pscustomobject]@{
        ClientPath = $executable
        DevRoot = $certificate
        EnvironmentKeys = @($allowed | Where-Object { $start.EnvironmentVariables.ContainsKey($_) })
        AutomaticLogin = $start.Arguments.Contains('--auto-login')
    }
    return
}
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Launch the compiled client on Windows.'
}
$client = [Diagnostics.Process]::Start($start)
if ($null -eq $client) { throw 'The client could not be started.' }
Write-Output "Client started (PID $($client.Id)). Sign in through its browser login button."
