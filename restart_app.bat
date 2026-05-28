@echo off
taskkill /IM twocents-desktop.exe /F >nul 2>&1
cd /d "%~dp0"
call cargo_build_msvc.bat build
if errorlevel 1 exit /b 1
start "" "%~dp0target\debug\twocents-desktop.exe"
