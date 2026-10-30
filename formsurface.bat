@echo off
cd /d "%~dp0"
if "%1"=="install" goto install
if "%1"=="engine" goto engine
if "%1"=="build" goto build
if "%1"=="serve" goto serve
if "%1"=="mcp" goto mcp
if "%1"=="claude" goto claude
if "%1"=="doctor" goto doctor
if "%1"=="install-desktop" goto installdesktop
if "%1"=="freeze" goto freeze
if "%1"=="help" goto help
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
:build
if not exist frontend\src\engine\pkg\fs_engine.js call :engine
pushd frontend
call npm run build || (popd & exit /b 1)
popd
goto :eof
:serve
if not exist frontend\dist\index.html call :build
pushd backend
call venv\Scripts\activate.bat
echo Open http://127.0.0.1:8766
uvicorn app.main:app --host 127.0.0.1 --port 8766
popd
goto :eof
:mcp
backend\venv\Scripts\python.exe -I mcp\server.py
goto :eof
:claude
where claude >nul 2>nul || (echo Claude Code is not installed: https://claude.com/claude-code & exit /b 1)
shift
claude --mcp-config "{\"mcpServers\":{\"formsurface\":{\"type\":\"stdio\",\"command\":\"%~dp0formsurface.bat\",\"args\":[\"mcp\"]}}}" --allowedTools mcp__formsurface %1 %2 %3 %4 %5 %6 %7 %8 %9
goto :eof
:doctor
echo FormSurface on Windows
where python >nul 2>nul && python --version || echo   python: missing (3.12 or newer)
where node >nul 2>nul && node --version || echo   node: missing (22.12 or newer)
where cargo >nul 2>nul && cargo --version || echo   rust: missing (https://rustup.rs)
where wasm-bindgen >nul 2>nul && wasm-bindgen --version || echo   wasm-bindgen: missing (formsurface.bat install)
if exist backend\venv\Scripts\activate.bat (echo   service venv: ready) else (echo   service venv: missing)
if exist frontend\node_modules (echo   editor packages: ready) else (echo   editor packages: missing)
if exist frontend\src\engine\pkg\fs_engine.js (echo   engine: built) else (echo   engine: not built)
where claude >nul 2>nul && (echo   claude code: found) || echo   claude code: not found
goto :eof
:installdesktop
call backend\venv\Scripts\activate.bat
pip install -r backend\requirements_desktop.txt
pushd frontend
call npm install
popd
goto :eof
:freeze
if not exist frontend\dist\index.html call :build
pushd backend
call venv\Scripts\activate.bat
for /f "tokens=2" %%t in ('rustc -vV ^| findstr host') do set TRIPLE=%%t
pyinstaller --noconfirm --clean --onefile --name formsurface-engine --distpath dist-engine --workpath build\pyinstaller --specpath build\pyinstaller --paths "%~dp0backend" --add-data "%~dp0frontend\dist;frontend_dist" --add-data "%~dp0mcp;mcp" --hidden-import app.agent.router --hidden-import app.agent.cmd_terminal --hidden-import app.agent.claude_code --hidden-import app.agent.cmd_chat --hidden-import app.agent.cmd_prefs --hidden-import winpty --collect-submodules uvicorn --collect-submodules app --collect-submodules mcp.server --collect-submodules mcp.shared --collect-submodules mcp_types --collect-submodules httpx --collect-all PyOpenColorIO "%~dp0backend\formsurface_engine.py" || (popd & exit /b 1)
dist-engine\formsurface-engine.exe --selfcheck || (popd & exit /b 1)
if not exist "%~dp0src-tauri\binaries" mkdir "%~dp0src-tauri\binaries"
copy /y dist-engine\formsurface-engine.exe "%~dp0src-tauri\binaries\formsurface-engine-%TRIPLE%.exe"
popd
goto :eof
:help
echo formsurface.bat [command]
echo   (none)          boot the service (8766) and the editor (5176) in two windows
echo   install         Rust wasm target, wasm-bindgen, service venv, editor packages, engine
echo   engine          rebuild the Rust engine
echo   build           build the engine and the editor into frontend\dist
echo   serve           serve the built editor from the service on 8766
echo   mcp             the MCP server over stdio
echo   claude [ARGS]   Claude Code connected to the formsurface MCP server
echo   doctor          check the toolchain and builds
echo   install-desktop PyInstaller and the Tauri command line
echo   freeze          freeze the service into src-tauri\binaries
goto :eof
