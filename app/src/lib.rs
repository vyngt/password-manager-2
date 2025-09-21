pub mod commands;
pub mod config;
pub mod interface;
pub mod store;

use tokio::sync::Mutex;

use store::registry::Registry;
use store::state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let settings = config::settings::Settings::from_env(app);
            settings.init_home_dir();

            let registry =
                tauri::async_runtime::block_on(async { Registry::from_settings(&settings).await });
            app.manage(Mutex::new(AppState { registry, settings }));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::vault::unlock_vault])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
