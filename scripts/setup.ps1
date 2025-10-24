#requires -Version 5.1
<#!
Mail Agent (Gmail Tray Notifier) — dependency checker/installer for Windows

This script verifies and helps install prerequisites needed to build and run the Tauri app.
It does NOT download big toolchains without your confirmation. Run from repo root:

  pwsh -File .\scripts\setup.ps1

Optional flags:
  -Auto        Try to install missing parts automatically using winget/choco/scoop if available.
  -Dev         After checks pass, run `cargo tauri dev`.
  -Build       After checks pass, run `cargo tauri build`.

Note: Running installers requires internet access.
!#>
param(
  [switch]$Auto,
  [switch]$Dev,
  [switch]$Build
)

$RepoRoot = (Resolve-Path "$PSScriptRoot\..\").Path
$ReleaseDir = Join-Path $RepoRoot 'release'

function Copy-ArtifactsToRelease {
  try {
    New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null
    $targetDir = Join-Path $RepoRoot 'src-tauri\target\release'
    if (Test-Path $targetDir) {
      Write-Header "Copying build artifacts to release/"
      # Copy main binaries
      Get-ChildItem -Path $targetDir -File -Include '*.exe','*.dll','*.pdb' -ErrorAction SilentlyContinue | ForEach-Object {
        Copy-Item -Path $_.FullName -Destination $ReleaseDir -Force
      }
      # Copy bundles (msi/nsis/etc.) preserving subfolders
      $bundleDir = Join-Path $targetDir 'bundle'
      if (Test-Path $bundleDir) {
        Copy-Item -Path $bundleDir -Destination $ReleaseDir -Recurse -Force
      }
    } else {
      Write-Warning "Build output directory not found: $targetDir"
    }
  } catch {
    Write-Warning "Failed to copy artifacts to release/: $_"
  }
}

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Write-Header($text) {
  Write-Host "`n=== $text ===" -ForegroundColor Cyan
}

function Test-Command($name) {
  return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

function Invoke-WithInfo {
  param(
    [Parameter(Mandatory=$true)][string]$cmd,
    [Parameter(ValueFromRemainingArguments=$true)][string[]]$Arguments
  )
  $argsText = if ($Arguments) { ($Arguments -join ' ') } else { '' }
  Write-Host "> $cmd $argsText" -ForegroundColor DarkGray
  # Use Start-Process for robust argument passing on Windows PowerShell 5.1
  $psi = @{
    FilePath     = $cmd
    ArgumentList = $Arguments
    NoNewWindow  = $true
    Wait         = $true
  }
  Start-Process @psi
}

function Suggest($text) { Write-Host $text -ForegroundColor Yellow }

# Detect package managers
$HasWinget = Test-Command winget
$HasChoco  = Test-Command choco
$HasScoop  = Test-Command scoop

Write-Header "Checking prerequisites"

# 1) Node.js and npm
$NodeOk = Test-Command node
$NpmOk  = Test-Command npm
if ($NodeOk) { Write-Host "Node: $(node -v)" } else { Write-Host "Node: not found" -ForegroundColor Red }
if ($NpmOk)  { Write-Host "npm:  $(npm -v)" }  else { Write-Host "npm:  not found" -ForegroundColor Red }

if (-not $NodeOk -or -not $NpmOk) {
  Suggest "Node.js + npm are required. Install one of:" 
  if ($HasWinget) { Write-Host "  winget install OpenJS.NodeJS.LTS" }
  if ($HasChoco)  { Write-Host "  choco install nodejs-lts -y" }
  if ($HasScoop)  { Write-Host "  scoop install nodejs-lts" }
  Write-Host "Or download from: https://nodejs.org/en/download" -ForegroundColor DarkGray
  if ($Auto) {
    try {
      if ($HasWinget) { Invoke-WithInfo winget @('install','-e','--id','OpenJS.NodeJS.LTS') }
      elseif ($HasChoco) { Invoke-WithInfo choco @('install','nodejs-lts','-y') }
      elseif ($HasScoop) { Invoke-WithInfo scoop @('install','nodejs-lts') }
    } catch { Write-Warning "Automatic Node.js install failed: $_" }
  }
}

# 2) Rust toolchain (rustup + cargo)
$RustupOk = Test-Command rustup
$CargoOk  = Test-Command cargo
if ($RustupOk) { Write-Host "rustup: $(rustup -V)" } else { Write-Host "rustup: not found" -ForegroundColor Red }
if ($CargoOk)  { Write-Host "cargo:  $(cargo -V)" }  else { Write-Host "cargo:  not found" -ForegroundColor Red }

if (-not $RustupOk -or -not $CargoOk) {
  Suggest "Rust toolchain is required. Install via:" 
  if ($HasWinget) { Write-Host "  winget install Rustlang.Rustup" }
  if ($HasChoco)  { Write-Host "  choco install rustup.install -y" }
  Write-Host "Or from: https://rustup.rs" -ForegroundColor DarkGray
  if ($Auto) {
    try {
      if ($HasWinget) { Invoke-WithInfo winget @('install','-e','--id','Rustlang.Rustup') }
      elseif ($HasChoco) { Invoke-WithInfo choco @('install','rustup.install','-y') }
    } catch { Write-Warning "Automatic Rust install failed: $_" }
  }
}

# Ensure the MSVC toolchain is installed
$MsvcOk = $false
try {
  $default = (& rustup show active-toolchain 2>$null)
  if ($default -match 'stable.*-pc-windows-msvc') { $MsvcOk = $true }
} catch {}
if (-not $MsvcOk) {
  Suggest "Ensure MSVC toolchain is installed: rustup toolchain install stable-x86_64-pc-windows-msvc"
  if ($Auto -and $RustupOk) {
    try { Invoke-WithInfo rustup @('toolchain','install','stable-x86_64-pc-windows-msvc') } catch { Write-Warning "Automatic MSVC toolchain install failed: $_" }
  }
}

# 3) Visual Studio Build Tools (C++ build tools) for native crates
$VsWhere = "$Env:ProgramFiles(x86)\Microsoft Visual Studio\Installer\vswhere.exe"
$HaveVSBT = Test-Path $VsWhere -PathType Leaf
if ($HaveVSBT) {
  Write-Host "VS Build Tools: detected (via vswhere)"
} else {
  Suggest "C++ Build Tools recommended (for native crates). Install one of:" 
  if ($HasWinget) { Write-Host "  winget install Microsoft.VisualStudio.2022.BuildTools" }
  if ($HasChoco)  { Write-Host "  choco install visualstudio2022buildtools -y" }
  Write-Host "Then select: Desktop development with C++" -ForegroundColor DarkGray
}

# 4) WebView2 Runtime (required by Tauri)
$WebView2Reg = 'HKLM:SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$HaveWV2 = Test-Path $WebView2Reg
if ($HaveWV2) { Write-Host "WebView2 Runtime: detected" } else {
  Suggest "WebView2 Runtime is required. Install via:" 
  if ($HasWinget) { Write-Host "  winget install Microsoft.EdgeWebView2Runtime" }
  if ($HasChoco)  { Write-Host "  choco install microsoft-edge-webview2-runtime -y" }
  Write-Host "Or download: https://developer.microsoft.com/en-us/microsoft-edge/webview2/" -ForegroundColor DarkGray
  if ($Auto -and $HasWinget) {
    try { Invoke-WithInfo winget @('install','-e','--id','Microsoft.EdgeWebView2Runtime') } catch { Write-Warning "Automatic WebView2 install failed: $_" }
  }
}

# 5) Tauri CLI (cargo-tauri or npm @tauri-apps/cli)
# Prefer checking for the cargo plugin binary directly; calling `cargo tauri` may not throw on failure
$CargoTauriOk = Test-Command 'cargo-tauri'
$NpmTauriOk = $false
if ($NpmOk) {
  try {
    $ver = (npm ls -g --depth=0 @tauri-apps/cli 2>$null)
    if ($LASTEXITCODE -eq 0 -and $ver -match '@tauri-apps/cli@') { $NpmTauriOk = $true }
  } catch { $NpmTauriOk = $false }
}
if ($CargoTauriOk -or $NpmTauriOk) {
  $tauriSources = @()
  if ($CargoTauriOk) { $tauriSources += 'cargo-tauri' }
  if ($NpmTauriOk) { $tauriSources += '@tauri-apps/cli' }
  Write-Host ("Tauri CLI: detected ({0})" -f ($tauriSources -join ', '))
} else {
  Suggest "Tauri CLI not found. Install one of:" 
  Write-Host "  cargo install tauri-cli" 
  if ($NpmOk) { Write-Host "  npm i -g @tauri-apps/cli" }
  if ($Auto) {
    try {
      if ($CargoOk) { Invoke-WithInfo cargo @('install','tauri-cli') }
      elseif ($NpmOk) { Invoke-WithInfo npm @('i','-g','@tauri-apps/cli') }
    } catch { Write-Warning "Automatic Tauri CLI install failed: $_" }
  }
}

# Summary (re-check after possible Auto installs)
Write-Header "Summary"
# Re-detect Tauri and WebView2 to reflect changes made during -Auto
$CargoTauriOk = Test-Command 'cargo-tauri'
$NpmTauriOk = $false
if (Test-Command npm) {
  try {
    $ver = (npm ls -g --depth=0 @tauri-apps/cli 2>$null)
    if ($LASTEXITCODE -eq 0 -and $ver -match '@tauri-apps/cli@') { $NpmTauriOk = $true }
  } catch { $NpmTauriOk = $false }
}
$HaveWV2 = Test-Path 'HKLM:SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'

$ok = @{}
$ok.Node   = Test-Command node
$ok.Npm    = Test-Command npm
$ok.Rustup = Test-Command rustup
$ok.Cargo  = Test-Command cargo
$ok.WV2    = $HaveWV2
$ok.Tauri  = ($CargoTauriOk -or $NpmTauriOk)
$missing = @()
foreach ($k in $ok.Keys) { if (-not $ok[$k]) { $missing += $k } }
if ($missing.Count -eq 0) {
  Write-Host "All core dependencies look good." -ForegroundColor Green
} else {
  Write-Warning ("Missing: " + ($missing -join ', '))
}

# Offer to run dev/build
if ($Dev -or $Build) {
  if ($missing.Count -gt 0) {
    Write-Warning "Some dependencies are missing. Dev/Build may fail."
  }
  Push-Location "$PSScriptRoot\..\src-tauri"
  try {
    if ($Dev) { Invoke-WithInfo cargo @('tauri','dev') }
    if ($Build) {
      Invoke-WithInfo cargo @('tauri','build')
      # After successful build, copy artifacts to the root release/ folder
      Pop-Location
      Copy-ArtifactsToRelease
      Push-Location "$PSScriptRoot\..\src-tauri"  # restore location for symmetry
    }
  } finally { Pop-Location }
}

Write-Host "Done."
