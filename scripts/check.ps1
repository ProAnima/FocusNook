<#
.SYNOPSIS
  Runs every quality gate from AGENTS.md in one command.

.DESCRIPTION
  Stages: frontend (tsc, eslint, vitest), desktop-rust (fmt, clippy, test), server (fmt, clippy, test).
  The desktop crate compiles SQLCipher with vendored OpenSSL, which needs a full Perl
  (Strawberry Perl). The script looks for one automatically (see Find-Perl); Git for
  Windows' MSYS Perl is not sufficient. Works in Windows PowerShell 5.1 and PowerShell 7.

.PARAMETER Only
  Run just the named stages: frontend, desktop-rust, server. Accepts several values,
  either as an array or as one comma-separated string (needed with `powershell -File`).

.PARAMETER KeepGoing
  Do not stop at the first failing stage; report everything at the end.

.PARAMETER Build
  Also run `npm run build` in the frontend stage.

.EXAMPLE
  scripts/check.ps1
  scripts/check.ps1 -Only frontend
  powershell -File scripts/check.ps1 -Only desktop-rust,server -KeepGoing

.NOTES
  Perl lookup order: $env:FOCUSNOOK_PERL (a directory containing perl.exe),
  C:\Strawberry\perl\bin, %USERPROFILE%\Tools\perl\perl\bin, then perl on PATH.
#>
[CmdletBinding()]
param(
  [string[]]$Only = @("frontend", "desktop-rust", "server"),
  [switch]$KeepGoing,
  [switch]$Build
)

$ErrorActionPreference = "Stop"
$validStages = @("frontend", "desktop-rust", "server")
$Only = @($Only | ForEach-Object { $_ -split "," } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$unknown = @($Only | Where-Object { $validStages -notcontains $_ })
if ($unknown.Count -gt 0) {
  Write-Host ("Unknown -Only value(s): {0}. Valid: {1}" -f ($unknown -join ", "), ($validStages -join ", ")) -ForegroundColor Red
  exit 2
}

$root = Split-Path -Parent $PSScriptRoot
$results = [System.Collections.Generic.List[object]]::new()

function Find-Perl {
  # A usable Perl must provide IPC::Cmd (missing from the MSYS Perl bundled with Git).
  $candidates = @(
    $env:FOCUSNOOK_PERL,
    "C:\Strawberry\perl\bin",
    (Join-Path $env:USERPROFILE "Tools\perl\perl\bin")
  )
  $fromPath = Get-Command perl -ErrorAction SilentlyContinue
  if ($fromPath) { $candidates += Split-Path $fromPath.Source }
  foreach ($dir in $candidates | Where-Object { $_ -and (Test-Path (Join-Path $_ "perl.exe")) }) {
    # Native stderr must not become a terminating error here (Windows PowerShell 5.1).
    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
      & (Join-Path $dir "perl.exe") -MIPC::Cmd -e1 2>$null | Out-Null
      $code = $LASTEXITCODE
    } catch {
      $code = 1
    } finally {
      $ErrorActionPreference = $previous
    }
    if ($code -eq 0) { return $dir }
  }
  return $null
}

function Show-Summary {
  Write-Host "`n=== Summary" -ForegroundColor Cyan
  foreach ($r in $results) {
    $mark = if ($r.Ok) { "PASS" } else { "FAIL" }
    $color = if ($r.Ok) { "Green" } else { "Red" }
    Write-Host ("{0}  [{1}] {2} ({3}s)" -f $mark, $r.Stage, $r.Step, $r.Seconds) -ForegroundColor $color
  }
}

function Add-Result([string]$Stage, [string]$Name, [bool]$Ok, [double]$Seconds) {
  $results.Add([pscustomobject]@{ Stage = $Stage; Step = $Name; Ok = $Ok; Seconds = [math]::Round($Seconds) })
  if (-not $Ok -and -not $KeepGoing) { Show-Summary; exit 1 }
}

function Invoke-Step([string]$Stage, [string]$Name, [string]$WorkDir, [scriptblock]$Command) {
  Write-Host "`n=== [$Stage] $Name" -ForegroundColor Cyan
  $timer = [System.Diagnostics.Stopwatch]::StartNew()
  Push-Location $WorkDir
  # Tools write progress to stderr; only the exit code decides success.
  $ErrorActionPreference = "Continue"
  $global:LASTEXITCODE = 0
  try {
    & $Command
    $ok = ($LASTEXITCODE -eq 0)
  } catch {
    Write-Host $_ -ForegroundColor Red
    $ok = $false
  } finally {
    Pop-Location
  }
  Add-Result $Stage $Name $ok $timer.Elapsed.TotalSeconds
}

function Invoke-RustStage([string]$Stage, [string]$WorkDir) {
  Invoke-Step $Stage "cargo fmt --check" $WorkDir { cargo fmt --check }
  Invoke-Step $Stage "cargo clippy" $WorkDir { cargo clippy --all-targets -- -D warnings }
  Invoke-Step $Stage "cargo test" $WorkDir { cargo test }
}

if ($Only -contains "frontend") {
  $app = Join-Path $root "apps\desktop"
  Invoke-Step "frontend" "tsc --noEmit" $app { npx tsc --noEmit }
  Invoke-Step "frontend" "eslint" $app { npm run lint }
  Invoke-Step "frontend" "vitest" $app { npm test }
  if ($Build) { Invoke-Step "frontend" "vite build" $app { npm run build } }
}

if ($Only -contains "desktop-rust") {
  $perl = Find-Perl
  if (-not $perl) {
    Write-Host "`nSKIPPED desktop-rust: no full Perl found (needs IPC::Cmd). Install Strawberry Perl or set FOCUSNOOK_PERL to the directory that contains perl.exe." -ForegroundColor Yellow
    Add-Result "desktop-rust" "perl prerequisite" $false 0
  } else {
    $savedPath = $env:PATH
    try {
      $env:PATH = "$perl;$env:PATH"
      Invoke-RustStage "desktop-rust" (Join-Path $root "apps\desktop\src-tauri")
    } finally {
      $env:PATH = $savedPath
    }
  }
}

if ($Only -contains "server") {
  Invoke-RustStage "server" (Join-Path $root "apps\server")
}

Show-Summary
if ($results | Where-Object { -not $_.Ok }) { exit 1 }
