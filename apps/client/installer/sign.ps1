# Code-signing hook (CLIENT-INSTALLER-0 section 6): sign.ps1 <file>.
# Owner decision D854: builds stay unsigned until external alpha, so no provider is configured
# and this hook signs nothing and exits 0. The provider-specific body is added only with the
# signing release job; credentials never enter the repository or a pull-request job.
param(
    [Parameter(Mandatory = $true)]
    [string] $File
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $File -PathType Leaf)) {
    throw "sign.ps1: no such file: $File"
}
Write-Host "sign.ps1: signing is not configured (D854); $File stays unsigned"
exit 0
