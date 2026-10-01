#Requires -Version 5.1
<#
.SYNOPSIS
    Build disktree on Windows, installing the Rust toolchain first if needed.

.DESCRIPTION
    One command to go from a fresh Windows checkout to target\release\disktree.exe:

      1. Finds Rust (rustup + cargo).
      2. If it is missing, installs it — with Chocolatey when the shell is
         elevated, otherwise into a self-contained .tools\ directory inside
         this repository, which needs no administrator rights.
      3. Makes sure the toolchain pinned in rust-toolchain.toml (and its clippy
         and rustfmt components) is installed.
      4. Imports the Visual Studio C++ environment so the MSVC linker and the
         Windows SDK's rc.exe are found.
      5. Runs the release build. With -Test it also runs the test suites.

    Nothing is installed system-wide unless Chocolatey is used, and the
    repository-local toolchain lives in a .tools\ directory that is gitignored.

.PARAMETER Check
    Only report what was found and what would happen; install nothing and
    build nothing.

.PARAMETER Test
    After a successful build, run `cargo test --workspace`.

.PARAMETER Toolchain
    Override the rustup channel. Default: the channel in rust-toolchain.toml.

.PARAMETER TargetDir
    Cargo target directory. Default: <repo>\target.

.EXAMPLE
    .\build.ps1

.EXAMPLE
    .\build.ps1 -Check

.EXAMPLE
    .\build.ps1 -Test
#>
[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$Test,
    [string]$Toolchain,
    [string]$TargetDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# Invoke-WebRequest is much faster without the progress bar.
$ProgressPreference = 'SilentlyContinue'

$RepoRoot = $PSScriptRoot
if (-not $RepoRoot) { $RepoRoot = (Get-Location).ProviderPath }

# Windows PowerShell 5.1 still defaults to old TLS protocols.
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
} catch {
    # .NET on newer PowerShell may not expose the setter; the default is fine.
}

# ---------------------------------------------------------------- output -----

function Write-Step([string]$Message) { Write-Host "==> $Message" -ForegroundColor Cyan }
function Write-Info([string]$Message) { Write-Host "    $Message" }
function Write-Ok([string]$Message)   { Write-Host "    $Message" -ForegroundColor Green }
function Write-Warn2([string]$Message) { Write-Host "    $Message" -ForegroundColor Yellow }

# --------------------------------------------------------------- helpers -----

function Get-EnvVar([string]$Name) {
    return [Environment]::GetEnvironmentVariable($Name, 'Process')
}

function Get-FirstCommand([string[]]$Names) {
    foreach ($name in $Names) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    return $null
}

function Add-Path([string]$Directory) {
    if (($env:Path -split ';') -notcontains $Directory) {
        $env:Path = "$Directory;$env:Path"
    }
}

function Get-HostTriple {
    # PROCESSOR_ARCHITECTURE is the *process* architecture: a 32-bit host on a
    # 64-bit machine reports x86 and hides AMD64 in PROCESSOR_ARCHITEW6432.
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($env:PROCESSOR_ARCHITEW6432) { $arch = $env:PROCESSOR_ARCHITEW6432 }
    switch ($arch) {
        'AMD64' { return 'x86_64-pc-windows-msvc' }
        'ARM64' { return 'aarch64-pc-windows-msvc' }
        'x86'   { return 'i686-pc-windows-msvc' }
        default { throw "unsupported processor architecture: $arch" }
    }
}

function Get-PinnedChannel {
    $file = Join-Path $RepoRoot 'rust-toolchain.toml'
    if (Test-Path $file) {
        $match = Select-String -Path $file -Pattern '^\s*channel\s*=\s*"([^"]+)"' |
            Select-Object -First 1
        if ($match) { return $match.Matches[0].Groups[1].Value }
    }
    return 'stable'
}

function Test-IsAdmin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# ---------------------------------------------------------------- rust -------

function Find-Rust {
    $rustup = Get-FirstCommand @('rustup')
    $cargo = Get-FirstCommand @('cargo')
    if ($rustup -and $cargo) {
        return [pscustomobject]@{ Cargo = $cargo; Rustup = $rustup; Source = 'PATH' }
    }

    # rustup is present but cargo is not on PATH (Chocolatey's rustup.install
    # shims rustup only): look where rustup keeps its proxies.
    if ($rustup) {
        $candidates = @()
        if ($env:CARGO_HOME) { $candidates += (Join-Path $env:CARGO_HOME 'bin\cargo.exe') }
        if ($env:USERPROFILE) { $candidates += (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe') }
        foreach ($candidate in $candidates) {
            if (Test-Path $candidate) {
                Add-Path (Split-Path $candidate)
                return [pscustomobject]@{ Cargo = $candidate; Rustup = $rustup; Source = 'rustup' }
            }
        }
    }

    $localCargoHome = Join-Path $RepoRoot '.tools\cargo'
    $localCargo = Join-Path $localCargoHome 'bin\cargo.exe'
    $localRustup = Join-Path $localCargoHome 'bin\rustup.exe'
    if ((Test-Path $localRustup) -and (Test-Path $localCargo)) {
        $env:CARGO_HOME = $localCargoHome
        $env:RUSTUP_HOME = Join-Path $RepoRoot '.tools\rustup'
        Add-Path (Split-Path $localCargo)
        return [pscustomobject]@{ Cargo = $localCargo; Rustup = $localRustup; Source = 'repo .tools' }
    }

    return $null
}

function Install-Rust {
    $choco = Get-FirstCommand @('choco')
    if ((Test-IsAdmin) -and $choco) {
        Write-Step 'Installing rustup with Chocolatey (rustup.install)'
        # Out-Host: a native command's stdout would otherwise be swallowed into
        # this function's return value and corrupt the object the caller wants.
        & $choco install rustup.install -y --no-progress | Out-Host
        if ($LASTEXITCODE -ne 0) {
            Write-Warn2 "Chocolatey exited with $LASTEXITCODE; falling back to a local install."
        } else {
            $found = Find-Rust
            if ($found) { return $found }
            Write-Warn2 'Chocolatey finished but rustup is not usable yet; falling back to a local install.'
        }
    } elseif (-not (Test-IsAdmin)) {
        Write-Info 'Not elevated: installing Rust inside this repository only (no admin needed).'
    }

    $tools = Join-Path $RepoRoot '.tools'
    $cargoHome = Join-Path $tools 'cargo'
    $rustupHome = Join-Path $tools 'rustup'
    New-Item -ItemType Directory -Force -Path $tools | Out-Null

    $env:CARGO_HOME = $cargoHome
    $env:RUSTUP_HOME = $rustupHome

    $triple = Get-HostTriple
    $url = "https://static.rust-lang.org/rustup/dist/$triple/rustup-init.exe"
    $init = Join-Path $tools 'rustup-init.exe'
    Write-Step "Downloading rustup-init.exe for $triple"
    Invoke-WebRequest -Uri $url -OutFile $init -UseBasicParsing

    Write-Step 'Installing the Rust toolchain (rustup-init -y --no-modify-path)'
    # Out-Host keeps rustup-init's "info:" stdout out of the return value below;
    # without it, `$rust = Install-Rust` becomes an array of info lines plus the
    # object, and `$rust.Cargo` then fails under Set-StrictMode.
    & $init -y --no-modify-path --default-toolchain none --profile minimal | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "rustup-init failed with exit code $LASTEXITCODE."
    }

    $cargo = Join-Path $cargoHome 'bin\cargo.exe'
    $rustup = Join-Path $cargoHome 'bin\rustup.exe'
    if (-not (Test-Path $cargo)) {
        throw "rustup-init finished but $cargo is missing."
    }
    Add-Path (Split-Path $cargo)
    return [pscustomobject]@{ Cargo = $cargo; Rustup = $rustup; Source = 'repo .tools' }
}

# ----------------------------------------------------------------- msvc -----

function Get-VsInstallPath {
    $programFilesX86 = Get-EnvVar 'ProgramFiles(x86)'
    if (-not $programFilesX86) { return $null }
    $vswhere = Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { return $null }

    $paths = & $vswhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath 2>$null
    if (-not $paths) { return $null }
    return (@($paths)[0]).Trim()
}

function Get-VcVarsPath {
    $vs = Get-VsInstallPath
    if (-not $vs) { return $null }
    $vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
    if (Test-Path $vcvars) { return $vcvars }
    return $null
}

function Test-MsvcEnvironment {
    # link.exe alone is not proof of MSVC: Git for Windows ships a coreutils
    # link.exe. The variables vcvars sets, or cl.exe, exist only in a real
    # MSVC shell.
    if ($env:VCToolsInstallDir -or $env:VCINSTALLDIR) { return $true }
    if (Get-Command cl.exe -ErrorAction SilentlyContinue) { return $true }
    return $false
}

function Enter-MsvcEnvironment {
    if (Test-MsvcEnvironment) {
        Write-Info 'MSVC environment already active.'
        return
    }
    $vcvars = Get-VcVarsPath
    if (-not $vcvars) {
        Write-Warn2 'Visual Studio C++ build tools were not found; the MSVC linker may be missing.'
        Write-Warn2 'Install "Visual Studio Build Tools" with the "Desktop development with C++" workload.'
        return
    }

    Write-Info "Importing the MSVC environment from $vcvars"
    # Through a temporary batch file: the quoting of
    # `cmd /c "call \"x.bat\" && set"` is fragile across PowerShell versions.
    $batch = Join-Path ([IO.Path]::GetTempPath()) ('disktree-vcvars-' + [guid]::NewGuid().ToString('N') + '.bat')
    $lines = $null
    $code = 1
    try {
        Set-Content -Path $batch -Value ("@call `"$vcvars`" >nul 2>&1`r`n@set") -Encoding ASCII
        $lines = & cmd.exe /c $batch
        $code = $LASTEXITCODE
    } finally {
        Remove-Item -Path $batch -Force -ErrorAction SilentlyContinue
    }
    if ($code -ne 0) {
        throw "vcvars64.bat failed with exit code $code."
    }
    foreach ($line in $lines) {
        $index = $line.IndexOf('=')
        if ($index -lt 1) { continue }
        $name = $line.Substring(0, $index)
        $value = $line.Substring($index + 1)
        # Skip cmd's pseudo-variables such as "=C:".
        if ($name -notmatch '^[A-Za-z_][A-Za-z0-9_()]*$') { continue }
        [Environment]::SetEnvironmentVariable($name, $value, 'Process')
    }

    if (Test-MsvcEnvironment) {
        Write-Ok 'MSVC environment is ready.'
    } else {
        Write-Warn2 'The MSVC environment could not be confirmed after importing vcvars64.bat.'
    }
}

# ----------------------------------------------------------------- main -----

$channel = if ($Toolchain) { $Toolchain } else { Get-PinnedChannel }
$rust = Find-Rust
$hasMsvc = (Test-MsvcEnvironment) -or [bool](Get-VcVarsPath)

Write-Step 'Prerequisites'
if ($rust) {
    Write-Ok "Rust:  $($rust.Cargo)  (from $($rust.Source))"
} else {
    Write-Warn2 'Rust:  not installed (would be installed now)'
}
if ($hasMsvc) {
    Write-Ok 'MSVC:  Visual Studio C++ build tools found'
} else {
    Write-Warn2 'MSVC:  not found — install the "Desktop development with C++" workload'
}
Write-Info "Toolchain: $channel"
Write-Info "Repo:      $RepoRoot"

if ($Check) {
    Write-Step 'Check only: nothing was installed and nothing was built.'
    return
}

if (-not $rust) {
    $rust = Install-Rust
    if (-not $rust) { throw 'Rust could not be installed.' }
}
Add-Path (Split-Path $rust.Cargo)

# Select the pinned channel explicitly, so cargo also works when the script is
# run from outside the repository (a repo-local install has no default channel).
$env:RUSTUP_TOOLCHAIN = $channel

Write-Step "Ensuring the pinned toolchain ($channel) and its components"
& $rust.Rustup toolchain install $channel --no-self-update -c clippy -c rustfmt
if ($LASTEXITCODE -ne 0) {
    throw "rustup toolchain install failed with exit code $LASTEXITCODE."
}
$cargoVersion = & $rust.Cargo --version
if ($LASTEXITCODE -ne 0) {
    throw "cargo could not run: $cargoVersion"
}
Write-Ok $cargoVersion

Write-Step 'Preparing the MSVC build environment'
Enter-MsvcEnvironment

if ($TargetDir) {
    if (-not [IO.Path]::IsPathRooted($TargetDir)) { $TargetDir = Join-Path $RepoRoot $TargetDir }
    $env:CARGO_TARGET_DIR = $TargetDir
}
$targetRoot = if ($TargetDir) { $TargetDir } else { Join-Path $RepoRoot 'target' }

Push-Location $RepoRoot
try {
    Write-Step 'Building disktree (release)'
    & $rust.Cargo build --release --locked -p disktree-app
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }

    if ($Test) {
        Write-Step 'Running the test suites'
        & $rust.Cargo test --workspace
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test failed with exit code $LASTEXITCODE."
        }
    }
} finally {
    Pop-Location
}

# The languages are read from beside the executable; put a copy there so a
# built disktree.exe finds them wherever it is run from.
$releaseDir = Join-Path $targetRoot 'release'
foreach ($file in Get-ChildItem -LiteralPath $RepoRoot -Filter 'disktree.*.i18n.txt' -File) {
    Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $releaseDir $file.Name) -Force
}
if (Test-Path (Join-Path $releaseDir 'disktree.de.i18n.txt')) {
    Write-Info "Languages copied to $releaseDir"
}

$exe = Join-Path $targetRoot 'release\disktree.exe'
if (Test-Path $exe) {
    Write-Ok "Built: $exe"
    Write-Info "Run it with:  & '$exe'"
} else {
    Write-Warn2 "The build succeeded but $exe was not found."
}
if (-not $Test) {
    Write-Info 'Add -Test to also run the test suites.'
}
