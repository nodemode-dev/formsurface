@echo off
cd /d "%~dp0"
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
pushd backend && call oc_install.bat && popd
pushd frontend && call oc_install.bat && popd
call formsurface.bat engine
