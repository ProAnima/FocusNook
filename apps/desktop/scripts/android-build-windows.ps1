param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug",
    [ValidateSet("apk", "aab")]
    [string]$Format = "apk",
    [ValidateSet("aarch64", "armv7")]
    [string[]]$Targets = @("aarch64", "armv7")
)

$ErrorActionPreference = "Stop"
$gitPerl = "C:\PROGRA~1\Git\usr\bin\perl.exe"
$gitBash = "C:\PROGRA~1\Git\usr\bin\bash.exe"
$strawberryLib = "C:\Strawberry\perl\lib"
$mingwMake = "C:\Strawberry\c\bin\mingw32-make.exe"
$ndkRoot = Get-ChildItem -LiteralPath (Join-Path $env:ANDROID_HOME "ndk") -Directory |
    Sort-Object Name -Descending |
    Select-Object -First 1
$clang = Join-Path $ndkRoot.FullName "toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe"
$llvmAr = Join-Path $ndkRoot.FullName "toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-ar.exe"
$llvmRanlib = Join-Path $ndkRoot.FullName "toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-ranlib.exe"

foreach ($path in @($gitPerl, $gitBash, $strawberryLib, $mingwMake, $clang, $llvmAr, $llvmRanlib)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Android SQLCipher build prerequisite is missing: $path"
    }
}

$toolDir = Join-Path $env:TEMP "focusnook-android-tools-v3"
New-Item -ItemType Directory -Force -Path $toolDir | Out-Null
Copy-Item -LiteralPath $mingwMake -Destination (Join-Path $toolDir "make.exe") -Force

$env:PATH = "$toolDir;C:\PROGRA~1\Git\usr\bin;$env:PATH"
$env:PERL = $gitPerl.Replace("\", "/")
$env:SHELL = $gitBash.Replace("\", "/")
$env:MAKEFLAGS = "SHELL=$($env:SHELL)"
$env:PERL5LIB = "/usr/lib/perl5/core_perl:/usr/share/perl5/core_perl:/c/Strawberry/perl/lib"
$env:MSYS2_ENV_CONV_EXCL = "PERL5LIB"
$env:CC_aarch64_linux_android = $clang.Replace("\", "/")
$env:AR_aarch64_linux_android = $llvmAr.Replace("\", "/")
$env:RANLIB_aarch64_linux_android = $llvmRanlib.Replace("\", "/")
$env:CC_armv7_linux_androideabi = $clang.Replace("\", "/")
$env:AR_armv7_linux_androideabi = $llvmAr.Replace("\", "/")
$env:RANLIB_armv7_linux_androideabi = $llvmRanlib.Replace("\", "/")

$arguments = @("run", "tauri", "--", "android", "build", "--ci", "--target") + $Targets
if ($Configuration -eq "Debug") {
    $arguments += "--debug"
}
$arguments += "--$Format"

& npm.cmd @arguments
if ($LASTEXITCODE -ne 0) {
    throw "Android build failed with exit code $LASTEXITCODE"
}
