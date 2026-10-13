@echo off
REM Build the OpenDex desktop application for Windows
REM
REM Usage:
REM   build\build-windows.bat              REM Debug build
REM   build\build-windows.bat release      REM Release build
REM
REM Output:
REM   target\release\opendex-desktop.exe   REM Release binary
REM   target\debug\opendex-desktop.exe     REM Debug build
REM   build\opendex-desktop-windows-x86_64.zip  REM Release archive

setlocal enabledelayedexpansion

set RELEASE=0
if /i "%~1"=="release" set RELEASE=1

echo === OpenDex Desktop - Windows Build ===

REM Check for required tools
where cargo >nul 2>&1 || (echo ERROR: cargo not found. Install Rust first. & exit /b 1)

REM Build
if %RELEASE% equ 1 (
    echo Building release...
    cargo build --release -p desktop-gui
    set BUILD_DIR=target\release
) else (
    echo Building debug...
    cargo build -p desktop-gui
    set BUILD_DIR=target\debug
)

set BINARY=%BUILD_DIR%\opendex-desktop.exe

if not exist "%BINARY%" (
    echo ERROR: Binary not found at %BINARY%
    exit /b 1
)

echo Binary built: %BINARY%
for %%A in ("%BINARY%") do echo Size: %%~zA bytes

REM Create release archive
if %RELEASE% equ 1 (
    set ARCHIVE=%~dp0opendex-desktop-windows-x86_64.zip
    echo Creating archive: %ARCHIVE%

    if exist "%ARCHIVE%" del "%ARCHIVE%"

    REM Use PowerShell to create zip
    powershell -Command ^
      "$ex='%ARCHIVE: =%%20%'; if(Test-Path $ex){Remove-Item $ex}; ^
      Compress-Archive -Path '%BINARY%' -DestinationPath $ex"

    if exist "%ARCHIVE%" (
        echo Archive created: %ARCHIVE%
        for %%A in ("%ARCHIVE%") do echo Size: %%~zA bytes
    )
)

echo.
echo === Build complete ===
echo Run: %BINARY%
