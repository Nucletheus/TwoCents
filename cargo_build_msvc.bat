@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" || exit /b 1
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d "%~dp0" || exit /b 1
cargo %*
exit /b %ERRORLEVEL%
