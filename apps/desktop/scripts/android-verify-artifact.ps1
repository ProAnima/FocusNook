param(
    [Parameter(Mandatory = $true)]
    [string]$Artifact,
    [switch]$AllowDebuggable
)

$ErrorActionPreference = "Stop"
$artifactPath = (Resolve-Path -LiteralPath $Artifact).Path
$extension = [IO.Path]::GetExtension($artifactPath).ToLowerInvariant()
if ($extension -notin @(".apk", ".aab")) {
    throw "Expected an APK or AAB artifact."
}

$buildTools = Get-ChildItem (Join-Path $env:ANDROID_HOME "build-tools") -Directory |
    Sort-Object Name -Descending |
    Select-Object -First 1
$apksigner = Join-Path $buildTools.FullName "apksigner.bat"
$apkanalyzer = (Get-Command apkanalyzer -ErrorAction Stop).Source
$jarsigner = (Get-Command jarsigner -ErrorAction Stop).Source

if ($extension -eq ".apk") {
    & $apksigner verify --verbose --print-certs $artifactPath
} else {
    & $jarsigner -verify -strict -certs $artifactPath
}
if ($LASTEXITCODE -ne 0) {
    throw "Artifact signature verification failed."
}

$applicationId = (& $apkanalyzer manifest application-id $artifactPath).Trim()
$versionName = (& $apkanalyzer manifest version-name $artifactPath).Trim()
$versionCode = (& $apkanalyzer manifest version-code $artifactPath).Trim()
$minSdk = (& $apkanalyzer manifest min-sdk $artifactPath).Trim()
$targetSdk = (& $apkanalyzer manifest target-sdk $artifactPath).Trim()
$debuggable = (& $apkanalyzer manifest debuggable $artifactPath).Trim()
$manifest = (& $apkanalyzer manifest print $artifactPath) -join "`n"
if ($applicationId -ne "com.proanima.focusnook") {
    throw "Unexpected application id: $applicationId"
}
if ($minSdk -ne "24" -or $targetSdk -ne "36") {
    throw "Unexpected SDK range: minSdk=$minSdk targetSdk=$targetSdk"
}
if (-not $AllowDebuggable -and $debuggable -ne "false") {
    throw "Store artifact is debuggable: $debuggable"
}
if ($manifest -notmatch 'android:allowBackup="false"') {
    throw "Store artifact must disable Android system backup for the local vault"
}

$permissions = (& $apkanalyzer manifest permissions $artifactPath) -join "`n"
$forbiddenPermissions = @(
    "android.permission.QUERY_ALL_PACKAGES",
    "android.permission.REQUEST_INSTALL_PACKAGES",
    "android.permission.MANAGE_EXTERNAL_STORAGE"
)
foreach ($permission in $forbiddenPermissions) {
    if ($permissions -match [regex]::Escape($permission)) {
        throw "Forbidden store permission found: $permission"
    }
}

$packages = (& $apkanalyzer dex packages $artifactPath) -join "`n"
foreach ($forbiddenPackage in @("com.google.android.gms.auth", "com.proanima.googleauth")) {
    if ($packages -match [regex]::Escape($forbiddenPackage)) {
        throw "GMS-only package found in store artifact: $forbiddenPackage"
    }
}

$files = (& $apkanalyzer files list $artifactPath) -join "`n"
foreach ($abi in @("arm64-v8a", "armeabi-v7a")) {
    if ($files -notmatch "(?m)/lib/$([regex]::Escape($abi))/libdesktop_lib\.so$") {
        throw "Required ABI is absent from the artifact: $abi"
    }
}

$hash = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash
Write-Host "Verified $applicationId version $versionName ($versionCode), SDK $minSdk-$targetSdk, debuggable=$debuggable"
Write-Host "SHA256 $hash"
Write-Host "Permissions:"
Write-Host $permissions
