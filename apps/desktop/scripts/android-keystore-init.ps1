param(
    [Parameter(Mandatory = $true)]
    [string]$DistinguishedName,
    [string]$Alias = "focusnook-release"
)

$ErrorActionPreference = "Stop"
$androidRoot = Resolve-Path (Join-Path $PSScriptRoot "..\src-tauri\gen\android")
$keystorePath = Join-Path $androidRoot "release-key.jks"
$propertiesPath = Join-Path $androidRoot "keystore.properties"
$keytool = (Get-Command keytool -ErrorAction Stop).Source

if ((Test-Path -LiteralPath $keystorePath) -or (Test-Path -LiteralPath $propertiesPath)) {
    throw "Release key files already exist. Refusing to overwrite permanent signing identity."
}

function Read-PlainPassword([string]$Prompt) {
    $secure = Read-Host $Prompt -AsSecureString
    $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer)
    }
}

$password = Read-PlainPassword "New release-key password"
$confirmation = Read-PlainPassword "Repeat release-key password"
if ($password.Length -lt 16) {
    throw "Use a release-key password of at least 16 characters."
}
if ($password -cne $confirmation) {
    throw "Passwords do not match."
}

& $keytool -genkeypair -v -keystore $keystorePath -storetype PKCS12 -storepass $password `
    -alias $Alias -keypass $password -keyalg RSA -keysize 4096 -validity 10000 `
    -dname $DistinguishedName
if ($LASTEXITCODE -ne 0) {
    throw "keytool failed with exit code $LASTEXITCODE"
}

[IO.File]::WriteAllLines($propertiesPath, @(
    "password=$password",
    "keyAlias=$Alias",
    "storeFile=release-key.jks"
), [Text.UTF8Encoding]::new($false))

Write-Host "Release signing identity created. Back up both files in an encrypted offline vault."
& $keytool -list -v -keystore $keystorePath -storepass $password -alias $Alias |
    Select-String -Pattern "SHA256:|Valid from:|Owner:"
