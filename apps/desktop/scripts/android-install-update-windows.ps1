param(
    [Parameter(Mandatory = $true)]
    [string]$Artifact
)

$ErrorActionPreference = "Stop"
$artifactPath = (Resolve-Path -LiteralPath $Artifact).Path
$adb = (Get-Command adb -ErrorAction Stop).Source
$devices = @(& $adb devices | Select-String "device$")
if ($devices.Count -ne 1) {
    throw "Connect exactly one Android device before updating."
}

& $adb install -r $artifactPath
if ($LASTEXITCODE -ne 0) {
    throw @"
In-place update failed. FocusNook was not uninstalled and its local data was not cleared.
Do not uninstall the existing app. Use an APK signed with the same certificate and a higher versionCode,
or first confirm that server sync contains the user's operations.
"@
}

Write-Host "FocusNook updated in place; Android app data was preserved."
