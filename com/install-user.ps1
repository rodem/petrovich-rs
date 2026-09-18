#Requires -Version 5.1
<#
.SYNOPSIS
  Per-user регистрация petrovich-com (Padeg.Declension) БЕЗ прав администратора.

.DESCRIPTION
  Вызывает DllRegisterServer обеих DLL через rundll32. Сама DLL пишет ТОЛЬКО
  HKCU\Software\Classes (64-битный вид — через 64-битный rundll32,
  Wow6432Node-вид для x86-клиентов — через 32-битный rundll32 из SysWOW64).
  HKLM не трогается, UAC-запросов нет.

.PARAMETER Configuration
  Release (по умолчанию) или Debug — откуда брать DLL.

.PARAMETER Test
  После регистрации создать объект и прогнать 4 метода (как в test_com.ps1).

.EXAMPLE
  .\install-user.ps1
  .\install-user.ps1 -Configuration Debug -Test
#>
[CmdletBinding()]
param(
    [ValidateSet("Release", "Debug")]
    [string]$Configuration = "Release",
    [switch]$Test
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Dll64 = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\$Configuration\petrovich_com.dll"
$Dll32 = Join-Path $RepoRoot "target\i686-pc-windows-msvc\$Configuration\petrovich_com.dll"

function Invoke-Register([string]$Rundll, [string]$Dll, [string]$Entry) {
    if (-not (Test-Path $Dll)) {
        Write-Warning "Нет $Dll — пропускаю."
        return $false
    }
    Write-Host "rundll32 $Entry : $Dll"
    $p = Start-Process -FilePath $Rundll -ArgumentList "`"$Dll`", $Entry" -Wait -PassThru -NoNewWindow
    if ($p.ExitCode -ne 0) {
        throw "$Entry завершился с кодом $($p.ExitCode) для $Dll"
    }
    return $true
}

$windir = $env:SystemRoot
$did64 = $false
$did32 = $false

if ([Environment]::Is64BitOperatingSystem) {
    # 64-битный rundll32 -> обычный вид HKCU\Software\Classes\CLSID.
    $did64 = Invoke-Register (Join-Path $windir "System32\rundll32.exe") $Dll64 "DllRegisterServer"
    # 32-битный rundll32 -> Wow6432Node-вид для x86-клиентов (1С x86, VBS).
    $did32 = Invoke-Register (Join-Path $windir "SysWOW64\rundll32.exe") $Dll32 "DllRegisterServer"
} else {
    # 32-битная Windows: один вид реестра, одна DLL.
    $did32 = Invoke-Register (Join-Path $windir "System32\rundll32.exe") $Dll32 "DllRegisterServer"
}

if (-not ($did64 -or $did32)) { throw "Не зарегистрирована ни одна DLL." }

Write-Host "OK: Padeg.Declension зарегистрирован per-user (HKCU, без админа)."

if ($Test) {
    & (Join-Path $PSScriptRoot 'test\test_com.ps1')
}
