@echo off
cd /d "%~dp0"
where cargo >nul 2>nul || (echo [formsurface] error: Rust is required, install it from https://rustup.rs 1>&2 & exit /b 1)
rustup target add wasm32-unknown-unknown
if errorlevel 1 goto failed
cargo install wasm-bindgen-cli --version 0.2.129 --locked
if errorlevel 1 goto failed
pushd backend
call oc_install.bat
if errorlevel 1 (popd & goto failed)
popd
pushd frontend
call oc_install.bat
if errorlevel 1 (popd & goto failed)
popd
call formsurface.bat engine
if errorlevel 1 goto failed
echo FormSurface installed. Run formsurface.bat
exit /b 0
:failed
echo [formsurface] error: the install stopped at the step above 1>&2
exit /b 1
