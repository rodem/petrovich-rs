@echo off
rem Per-user регистрация petrovich-com БЕЗ прав администратора и БЕЗ PowerShell.
rem Использование: install-user.cmd [Release ^| Debug]
rem Пишет только HKCU\Software\Classes (HKLM не трогается, UAC-запросов нет).
setlocal
set CFG=%~1
if "%CFG%"=="" set CFG=Release
set ROOT=%~dp0..
set DLL64=%ROOT%\target\x86_64-pc-windows-msvc\%CFG%\petrovich_com.dll
set DLL32=%ROOT%\target\i686-pc-windows-msvc\%CFG%\petrovich_com.dll

rem 64-битный rundll32: из 32-битного cmd.exe System32 перенаправляется в
rem SysWOW64, поэтому явно используем Sysnative, если он есть.
set RUNDLL64=%SystemRoot%\System32\rundll32.exe
if /i "%PROCESSOR_ARCHITECTURE%"=="x86" (
  if defined PROCESSOR_ARCHITEW6432 set RUNDLL64=%SystemRoot%\Sysnative\rundll32.exe
)
set RUNDLL32=%SystemRoot%\SysWOW64\rundll32.exe
if not exist "%RUNDLL32%" set RUNDLL32=

set DID_ANY=0
if exist "%DLL64%" (
  echo rundll32 DllRegisterServer : %DLL64%
  "%RUNDLL64%" "%DLL64%", DllRegisterServer
  if errorlevel 1 goto regfail
  set DID_ANY=1
) else (
  echo Нет %DLL64% — пропускаю x64.
)
if defined RUNDLL32 (
  if exist "%DLL32%" (
    echo rundll32 DllRegisterServer : %DLL32%
    "%RUNDLL32%" "%DLL32%", DllRegisterServer
    if errorlevel 1 goto regfail
    set DID_ANY=1
  ) else (
    echo Нет %DLL32% — пропускаю x86.
  )
) else (
  echo Нет 32-битного rundll32 — пропускаю x86.
)
if "%DID_ANY%"=="0" (
  echo ОШИБКА: не зарегистрирована ни одна DLL.
  exit /b 1
)
echo OK: Padeg.Declension зарегистрирован per-user (HKCU, без админа).
exit /b 0

:regfail
echo ОШИБКА: DllRegisterServer завершился с кодом %ERRORLEVEL%.
exit /b 1
