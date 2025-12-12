@echo off
REM Set the variable for the virtual environment directory
set VENV_DIR=venv

REM Check if Python is installed
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Python is not installed. Please install Python and add it to PATH.
    exit /b 1
)

REM Create the virtual environment
python -m venv %VENV_DIR%

REM Check if the virtual environment was created successfully
if exist %VENV_DIR%\Scripts\activate (
    echo Virtual environment successfully created in the %VENV_DIR% folder.
) else (
    echo Failed to create virtual environment.
    exit /b 1
)

REM Activate the virtual environment
call %VENV_DIR%\Scripts\activate

REM Check if requirements.txt exists
if exist requirements.txt (
    REM Install dependencies from requirements.txt
    pip install -r requirements.txt
    echo Dependencies installed from requirements.txt.
) else (
    echo requirements.txt not found. Skipping dependency installation.
)

behave

REM Keep the command window open
cmd /k
