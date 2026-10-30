@echo off
setlocal
cd /d "%~dp0.."
if not exist backend\venv\Scripts\activate.bat (echo [formsurface] error: service venv missing, run formsurface.bat install 1>&2 & exit /b 1)
if not exist frontend\node_modules\.bin\tauri.cmd (echo [formsurface] error: the Tauri command line is missing, run formsurface.bat install-desktop 1>&2 & exit /b 1)
echo [formsurface] == building the WebGPU engine (Rust to WebAssembly)
call formsurface.bat engine
if errorlevel 1 exit /b 1
echo [formsurface] == building the editor
pushd frontend
call npm run build
if errorlevel 1 (popd & exit /b 1)
popd
echo [formsurface] == freezing the service into the app's sidecar
call formsurface.bat freeze
if errorlevel 1 exit /b 1
set TRIPLE=
for /f "tokens=2" %%t in ('rustc -vV ^| findstr host') do set TRIPLE=%%t
if not exist "src-tauri\binaries\formsurface-engine-%TRIPLE%.exe" (echo [formsurface] error: src-tauri\binaries\formsurface-engine-%TRIPLE%.exe is missing after the freeze 1>&2 & exit /b 1)
echo [formsurface] == building the desktop app (the first build compiles several hundred Rust crates)
call frontend\node_modules\.bin\tauri.cmd build %*
if errorlevel 1 (echo [formsurface] error: tauri build failed 1>&2 & exit /b 1)
echo [formsurface] installers: src-tauri\target\release\bundle
exit /b 0
