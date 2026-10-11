@echo off
cd /d "%~dp0"
if "%1"=="install" goto install
if "%1"=="engine" goto engine
if not exist frontend\src\engine\pkg\fs_engine.js call :engine
start "FormSurface backend" cmd /k "cd backend && boot_backend.bat"
start "FormSurface frontend" cmd /k "cd frontend && boot_frontend.bat"
goto :eof
:install
call formsurface_oc_install.bat
goto :eof
:engine
pushd native
cargo build --release --target wasm32-unknown-unknown -p fs-engine || exit /b 1
popd
wasm-bindgen --target web --out-dir frontend\src\engine\pkg native\target\wasm32-unknown-unknown\release\fs_engine.wasm
goto :eof
