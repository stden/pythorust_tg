@echo off
REM Set up the project environment with uv (https://docs.astral.sh/uv/)

REM Check if uv is installed
uv --version >nul 2>&1
if %errorlevel% neq 0 (
    echo uv is not installed. Install from https://docs.astral.sh/uv/ and add it to PATH.
    exit /b 1
)

REM Create the virtual environment and install dependencies from pyproject.toml
uv sync
echo Dependencies installed with uv.

uv run behave

REM Keep the command window open
cmd /k
