#Requires -Version 5.1
<#
.SYNOPSIS
  Удаление per-user регистрации petrovich-com (без прав администратора).
#>
[CmdletBinding()]
param(
    [ValidateSet("Release", "Debug")]
    [string]$Configuration = "Release"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Dll64 = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\$Configuration\petrovich_com.dll"
$Dll32 = Join-Path $RepoRoot "target\i686-pc-windows-msvc\$Configuration\petrovich_com.dll"

function Invoke-Unregister([string]$Rundll, [string]$Dll) {
    if (-not (Test-Path $Dll)) {
        Write-Warning "Нет $Dll — пропускаю."
        return
    }
    Write-Host "rundll32 DllUnregisterServer : $Dll"
    $p = Start-Process -FilePath $Rundll -ArgumentList "`"$Dll`", DllUnregisterServer" -Wait -PassThru -NoNewWindow
    if ($p.ExitCode -ne 0) {
        throw "DllUnregisterServer завершился с кодом $($p.ExitCode) для $Dll"
    }
}

$windir = $env:SystemRoot
if ([Environment]::Is64BitOperatingSystem) {
    Invoke-Unregister (Join-Path $windir "System32\rundll32.exe") $Dll64
    Invoke-Unregister (Join-Path $windir "SysWOW64\rundll32.exe") $Dll32
} else {
    Invoke-Unregister (Join-Path $windir "System32\rundll32.exe") $Dll32
}

Write-Host "OK: регистрация снята."
