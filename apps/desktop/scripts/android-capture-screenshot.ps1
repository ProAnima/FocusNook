param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-z0-9-]+$')]
    [string]$Name,
    [string]$Serial
)

$ErrorActionPreference = "Stop"
$adb = (Get-Command adb -ErrorAction Stop).Source
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..")
$outputDir = Join-Path $repoRoot "docs\store-assets\screenshots\raw"
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
$output = Join-Path $outputDir "$Name.png"
$remote = "/sdcard/focusnook-$Name.png"
$serialArgs = if ($Serial) { @("-s", $Serial) } else { @() }

& $adb @serialArgs shell screencap -p $remote
if ($LASTEXITCODE -ne 0) { throw "Device screenshot failed" }
try {
    & $adb @serialArgs pull $remote $output
    if ($LASTEXITCODE -ne 0) { throw "Screenshot pull failed" }
} finally {
    & $adb @serialArgs shell rm -f $remote | Out-Null
}

Write-Host "Captured $output. Crop navigation chrome only; do not alter app content."
