const APP_COMMANDS: &[&str] = &["set_window_title", "app_ready", "text_edit", "listen_for_exit", "confirm_exit"];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to run the Tauri build step");
}
