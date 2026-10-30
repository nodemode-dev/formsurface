@echo off
cd /d "%~dp0"
where python >nul 2>nul || (echo [formsurface] error: Python 3.12 or newer is required 1>&2 & exit /b 1)
if not exist venv\Scripts\activate.bat python -m venv venv
if errorlevel 1 exit /b 1
call venv\Scripts\activate.bat
if errorlevel 1 exit /b 1
python -m pip install --upgrade pip
if errorlevel 1 exit /b 1
python -m pip install -r requirements.txt
if errorlevel 1 exit /b 1
exit /b 0
