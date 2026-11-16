@echo off
setlocal
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
if "%1"=="desktop" goto desktop
if "%1"=="help" goto help
if "%1"=="-h" goto help
if "%1"=="--help" goto help
if "%1"=="backend" goto backend
if "%1"=="frontend" goto frontend
if "%1"=="batch" goto batch
if "%1"=="script" goto script
if not "%1"=="" if not "%1"=="boot" (call :help & exit /b 2)
if not exist frontend\src\engine\pkg\fs_engine.js call :engine
start "FormSurface backend" cmd /k "cd backend && boot_backend.bat"
start "FormSurface frontend" cmd /k "cd frontend && boot_frontend.bat"
goto :eof
:install
call formsurface_oc_install.bat
goto :eof
:engine
pushd native
cargo build --release --target wasm32-unknown-unknown -p fs-engine || (popd & exit /b 1)
popd
wasm-bindgen --target web --out-dir frontend\src\engine\pkg native\target\wasm32-unknown-unknown\release\fs_engine.wasm || exit /b 1
goto :eof
:build
if not exist frontend\src\engine\pkg\fs_engine.js (call :engine & if errorlevel 1 exit /b 1)
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
:backend
call backend\boot_backend.bat
goto :eof
:frontend
call frontend\boot_frontend.bat
goto :eof
:batch
if not exist backend\venv\Scripts\python.exe (echo Run formsurface.bat install first 1>&2 & exit /b 1)
shift
backend\venv\Scripts\python.exe -I mcp\batch.py %1 %2 %3 %4 %5 %6 %7 %8 %9
goto :eof
:script
if not exist backend\venv\Scripts\python.exe (echo Run formsurface.bat install first 1>&2 & exit /b 1)
shift
backend\venv\Scripts\python.exe -I mcp\run_script.py %1 %2 %3 %4 %5 %6 %7 %8 %9
goto :eof
:claude
where claude >nul 2>nul || (echo Claude Code is not installed: https://claude.com/claude-code & exit /b 1)
set "FS_ROOT=%~dp0"& shift
if not defined MCP_TOOL_TIMEOUT set "MCP_TOOL_TIMEOUT=1800000"
claude --mcp-config "{\"mcpServers\":{\"formsurface\":{\"type\":\"stdio\",\"command\":\"%FS_ROOT:\=\\%formsurface.bat\",\"args\":[\"mcp\"]}}}" --allowedTools mcp__formsurface %1 %2 %3 %4 %5 %6 %7 %8 %9
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
if not exist backend\venv\Scripts\activate.bat (echo [formsurface] error: service venv missing, run formsurface.bat install 1>&2 & exit /b 1)
call backend\venv\Scripts\activate.bat
python -m pip install -r backend\requirements_desktop.txt || exit /b 1
pushd frontend
call npm install || (popd & exit /b 1)
popd
goto :eof
:freeze
if not exist frontend\dist\index.html (call :build & if errorlevel 1 exit /b 1)
if not exist backend\venv\Scripts\activate.bat (echo [formsurface] error: service venv missing, run formsurface.bat install 1>&2 & exit /b 1)
pushd backend
call venv\Scripts\activate.bat
set TRIPLE=
for /f "tokens=2" %%t in ('rustc -vV ^| findstr host') do set TRIPLE=%%t
if not defined TRIPLE (echo [formsurface] error: rustc not found, install Rust from https://rustup.rs 1>&2 & popd & exit /b 1)
pyinstaller --noconfirm --clean --onefile --name formsurface-engine --distpath dist-engine --workpath build\pyinstaller --specpath build\pyinstaller --paths "%~dp0backend" --add-data "%~dp0frontend\dist;frontend_dist" --add-data "%~dp0mcp;mcp" --hidden-import app.agent.router --hidden-import app.agent.cmd_terminal --hidden-import app.agent.claude_code --hidden-import app.agent.cmd_chat --hidden-import app.agent.cmd_prefs --hidden-import winpty --collect-all winpty --collect-submodules uvicorn --collect-submodules app --collect-submodules mcp.server --collect-submodules mcp.shared --collect-submodules mcp_types --collect-submodules httpx --collect-all PyOpenColorIO "%~dp0backend\formsurface_engine.py" || (popd & exit /b 1)
dist-engine\formsurface-engine.exe --selfcheck || (popd & exit /b 1)
if not exist "%~dp0src-tauri\binaries" mkdir "%~dp0src-tauri\binaries"
copy /y dist-engine\formsurface-engine.exe "%~dp0src-tauri\binaries\formsurface-engine-%TRIPLE%.exe" || (popd & exit /b 1)
popd
goto :eof
:desktop
if "%2"=="dev" goto desktopdev
if not "%2"=="build" (call :help & exit /b 2)
call setup\desktop_build.bat %3 %4 %5 %6 %7 %8 %9
goto :eof
:desktopdev
if not exist backend\venv\Scripts\activate.bat (echo [formsurface] error: service venv missing, run formsurface.bat install 1>&2 & exit /b 1)
if not exist frontend\node_modules\.bin\tauri.cmd (echo [formsurface] error: the Tauri command line is missing, run formsurface.bat install-desktop 1>&2 & exit /b 1)
set TRIPLE=
for /f "tokens=2" %%t in ('rustc -vV ^| findstr host') do set TRIPLE=%%t
if not defined TRIPLE (echo [formsurface] error: rustc not found, run formsurface.bat install-desktop 1>&2 & exit /b 1)
if not exist "src-tauri\binaries\formsurface-engine-%TRIPLE%.exe" (call :freeze & if errorlevel 1 exit /b 1)
call frontend\node_modules\.bin\tauri.cmd dev %3 %4 %5 %6 %7 %8 %9
goto :eof
:help
echo formsurface.bat [command]
echo   (none) ^| boot   boot the service (8766) and the editor (5176) in two windows
echo   install         Rust wasm target, wasm-bindgen, service venv, editor packages, engine
echo   engine          rebuild the Rust engine
echo   backend         boot the service only
echo   frontend        boot the editor only
echo   build           build the engine and the editor into frontend\dist
echo   serve           serve the built editor from the service on 8766
echo   mcp             the MCP server over stdio
echo   batch FILE      run a JSON batch script of formsurface_* tool calls against the linked editor
echo                   (--wait, --out, --pause, --keep-going, --headless [--size N]; nine words at most)
echo   script FILE     run a Python script that imports formsurface_api against the linked editor
echo                   (script --headless [--size N] FILE runs it without an open editor)
echo   claude [ARGS]   Claude Code connected to the formsurface MCP server
echo   doctor          check the toolchain and builds
echo   install-desktop PyInstaller and the Tauri command line
echo   freeze          freeze the service into src-tauri\binaries
echo   desktop dev [ARGS]    run the desktop app from source
echo   desktop build [ARGS]  build the engine, the editor, the sidecar and the desktop app;
echo                   ARGS such as --config FILE go to tauri build
echo   help            this text
goto :eof
