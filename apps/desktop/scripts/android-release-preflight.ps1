param(
    [switch]$AllowDirty,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$desktopRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$repoRoot = Resolve-Path (Join-Path $desktopRoot "..\..")
$tauriRoot = Join-Path $desktopRoot "src-tauri"
$serverRoot = Join-Path $repoRoot "apps\server"
$androidRoot = Join-Path $tauriRoot "gen\android"

if (-not $AllowDirty) {
    $dirty = git -C $repoRoot status --porcelain
    if ($dirty) {
        throw "Release preflight requires a clean Git worktree. Review and commit the intended release first."
    }
}

$tauriConfig = Get-Content -Raw -Encoding UTF8 (Join-Path $tauriRoot "tauri.conf.json") | ConvertFrom-Json
$package = Get-Content -Raw -Encoding UTF8 (Join-Path $desktopRoot "package.json") | ConvertFrom-Json
$cargoText = Get-Content -Raw -Encoding UTF8 (Join-Path $tauriRoot "Cargo.toml")
$cargoVersion = [regex]::Match($cargoText, '(?m)^version = "([^"]+)"').Groups[1].Value
if (($tauriConfig.version -ne $package.version) -or ($tauriConfig.version -ne $cargoVersion)) {
    throw "Version mismatch: Tauri=$($tauriConfig.version), npm=$($package.version), Cargo=$cargoVersion"
}

foreach ($path in @("keystore.properties", "release-key.jks")) {
    if (-not (Test-Path -LiteralPath (Join-Path $androidRoot $path))) {
        throw "Missing permanent release signing file: $path"
    }
}

& npm.cmd run lint
if ($LASTEXITCODE -ne 0) { throw "ESLint failed" }
& npx.cmd tsc --noEmit
if ($LASTEXITCODE -ne 0) { throw "TypeScript failed" }
& npm.cmd test -- --run
if ($LASTEXITCODE -ne 0) { throw "Frontend tests failed" }
& cargo clippy --manifest-path (Join-Path $tauriRoot "Cargo.toml") --all-targets
if ($LASTEXITCODE -ne 0) { throw "Desktop clippy failed" }
& cargo test --manifest-path (Join-Path $tauriRoot "Cargo.toml") -- --test-threads=1
if ($LASTEXITCODE -ne 0) { throw "Desktop Rust tests failed" }
& cargo clippy --manifest-path (Join-Path $serverRoot "Cargo.toml") --all-targets
if ($LASTEXITCODE -ne 0) { throw "Server clippy failed" }
& cargo test --manifest-path (Join-Path $serverRoot "Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "Server tests failed" }

if ($SkipBuild) {
    Write-Host "Preflight gates passed; release build skipped."
    exit 0
}

& (Join-Path $PSScriptRoot "android-build-windows.ps1") -Configuration Release -Format aab
if ($LASTEXITCODE -ne 0) { throw "Release AAB build failed" }
$artifact = Get-ChildItem (Join-Path $androidRoot "app\build\outputs\bundle") -Recurse -File -Filter *.aab |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $artifact) { throw "Release AAB was not found" }
& (Join-Path $PSScriptRoot "android-verify-artifact.ps1") -Artifact $artifact.FullName
if ($LASTEXITCODE -ne 0) { throw "Release artifact verification failed" }
Write-Host "Release candidate ready: $($artifact.FullName)"
