@echo off
rem Снятие per-user регистрации petrovich-com. Без прав администратора,
rem без PowerShell. Использование: uninstall-user.cmd [Release ^| Debug]
setlocal
set CFG=%~1
if "%CFG%"=="" set CFG=Release
set ROOT=%~dp0..
set DLL64=%ROOT%\target\x86_64-pc-windows-msvc\%CFG%\petrovich_com.dll
set DLL32=%ROOT%\target\i686-pc-windows-msvc\%CFG%\petrovich_com.dll

set RUNDLL64=%SystemRoot%\System32\rundll32.exe
if /i "%PROCESSOR_ARCHITECTURE%"=="x86" (
  if defined PROCESSOR_ARCHITEW6432 set RUNDLL64=%SystemRoot%\Sysnative\rundll32.exe
)
set RUNDLL32=%SystemRoot%\SysWOW64\rundll32.exe
if not exist "%RUNDLL32%" set RUNDLL32=

if exist "%DLL64%" (
  echo rundll32 DllUnregisterServer : %DLL64%
  "%RUNDLL64%" "%DLL64%", DllUnregisterServer
  if errorlevel 1 goto regfail
) else (
  echo Нет %DLL64% — пропускаю x64.
)
if defined RUNDLL32 (
  if exist "%DLL32%" (
    echo rundll32 DllUnregisterServer : %DLL32%
    "%RUNDLL32%" "%DLL32%", DllUnregisterServer
    if errorlevel 1 goto regfail
  ) else (
    echo Нет %DLL32% — пропускаю x86.
  )
)
echo OK: регистрация снята.
exit /b 0

:regfail
echo ОШИБКА: DllUnregisterServer завершился с кодом %ERRORLEVEL%.
exit /b 1
