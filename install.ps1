#Requires -Version 5.1
<#
.SYNOPSIS
    Install disktree: the release binary and its language files.

.DESCRIPTION
    Copies disktree.exe and every disktree.<lang>.i18n.txt beside it, so the
    program finds its languages at startup (it looks next to the executable
    first). Nothing else is installed; there is no service and no registry key.

    The binary is built first unless it already exists or -NoBuild is given.
    Use build.ps1 directly when Rust has to be installed as well.

.PARAMETER Prefix
    Install directory. Default: %LOCALAPPDATA%\Programs\disktree.

.PARAMETER NoBuild
    Do not build; install an already built target\release\disktree.exe.

.PARAMETER Uninstall
    Remove exactly what this script installed.

.EXAMPLE
    .\install.ps1

.EXAMPLE
    .\install.ps1 -Prefix 'C:\Tools\disktree'

.EXAMPLE
    .\install.ps1 -Uninstall
#>
[CmdletBinding()]
param(
    [string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\disktree'),
    [switch]$NoBuild,
    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = $PSScriptRoot
if (-not $RepoRoot) { $RepoRoot = (Get-Location).ProviderPath }

$binary = Join-Path $RepoRoot 'target\release\disktree.exe'
$languages = @(Get-ChildItem -LiteralPath $RepoRoot -Filter 'disktree.*.i18n.txt' -File |
    Sort-Object Name)

if ($Uninstall) {
    Remove-Item -LiteralPath (Join-Path $Prefix 'disktree.exe') -Force -ErrorAction SilentlyContinue
    foreach ($file in $languages) {
        Remove-Item -LiteralPath (Join-Path $Prefix $file.Name) -Force -ErrorAction SilentlyContinue
    }
    # Remove the directory only when this script emptied it.
    if ((Test-Path $Prefix) -and -not (Get-ChildItem -LiteralPath $Prefix -Force)) {
        Remove-Item -LiteralPath $Prefix -Force
    }
    Write-Host "removed $Prefix"
    return
}

if (-not $NoBuild -and -not (Test-Path $binary)) {
    $build = Join-Path $RepoRoot 'build.ps1'
    if (-not (Test-Path $build)) {
        throw "no $binary and no build.ps1; build disktree first"
    }
    Write-Host 'Building disktree (release)...'
    & $build
}

if (-not (Test-Path $binary)) {
    throw "$binary is still missing; the build did not produce it"
}

New-Item -ItemType Directory -Force -Path $Prefix | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $Prefix 'disktree.exe') -Force
foreach ($file in $languages) {
    Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $Prefix $file.Name) -Force
}

$languages | ForEach-Object { Write-Host "  language: $($_.Name)" }
Write-Host "installed: $(Join-Path $Prefix 'disktree.exe')"

$onPath = ($env:Path -split ';') -contains $Prefix
if (-not $onPath) {
    Write-Host "note: $Prefix is not on PATH; run it by full path or add it:"
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$Prefix', 'User')"
}
