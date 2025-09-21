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
        .invoke_handler(tauri::generate_handler![
            // Vault commands
            commands::vault::unlock_vault,
            commands::vault::change_key_vault,
            commands::vault::create_vault_item,
            commands::vault::get_vault_item,
            commands::vault::update_vault_item,
            commands::vault::delete_vault_item,
            commands::vault::list_vault_items,
            commands::vault::export_vault_items,
            commands::vault::import_vault_items,
            // Theme commands
            commands::theme::get_current_theme,
            commands::theme::create_color_scheme,
            commands::theme::get_color_scheme,
            commands::theme::update_color_scheme,
            commands::theme::delete_color_scheme,
            commands::theme::list_color_schemes,
            commands::theme::update_theme,
            // Utilities commands
            commands::utilities::generate_password,
            commands::utilities::read_from_file,
            commands::utilities::write_to_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
