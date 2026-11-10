@echo off
REM decompile-everything - Windows batch wrapper for the decompilation skill

setlocal enabledelayedexpansion

REM Get the directory of this script
set "SCRIPT_DIR=%~dp0"

REM Default to Python
set "PYTHON_CMD=python"

REM Check if decompile-everything is installed globally
where decompile-everything >nul 2>nul
if not errorlevel 1 (
    REM Use installed version
    decompile-everything %*
    exit /b %errorlevel%
)

REM Check if we have the source installation
if exist "%SCRIPT_DIR%pyproject.toml" (
    REM Run from source with proper Python path
    set "PYTHONPATH=%SCRIPT_DIR%;%PYTHONPATH%"
    %PYTHON_CMD% -m decompile_everything.commands %*
    exit /b %errorlevel%
)

REM Not installed
echo Error: decompile-everything not properly installed
echo Install with: pip install -e %SCRIPT_DIR%
exit /b 1