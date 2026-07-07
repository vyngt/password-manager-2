//! Vedge Tauri shell. Thin adapter over `vedge-core` use cases.

pub mod commands;
pub mod dto;
pub mod error;
pub mod pdf;
pub mod setup;
pub mod state;

#[cfg(test)]
pub(crate) mod test_support;

use tauri::Manager;

/// Entry point wired from `main.rs`.
///
/// Constructs the Tauri builder, registers plugins, runs
/// `setup::services::compose` to build [`state::AppState`], and declares
/// every `#[tauri::command]` in `invoke_handler!`.
///
/// `.run(...).expect(...)` mirrors the canonical Tauri bootstrap —
/// a failure here means the runtime itself couldn't start, which is
/// irrecoverable. Allowed inline so the rest of the crate keeps
/// `expect_used = deny`.
#[allow(
    clippy::too_many_lines,
    clippy::expect_used,
    clippy::large_stack_frames
)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_single_instance::init(|_, _, _| {}));
    }

    builder
        .setup(|app| {
            // `compose` is `!Send` (holds &tauri::App); run it on the
            // current thread via Tauri's async runtime. The error type of
            // `compose` is `Box<dyn Error + Send + Sync>`; tauri's setup
            // hook wants `Box<dyn Error>` — just re-box.
            let state = tauri::async_runtime::block_on(setup::services::compose(app))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ---- vault lifecycle + read queries ----
            commands::vault::create_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::is_unlocked,
            commands::vault::list_entries,
            commands::vault::list_trashed,
            commands::vault::search,
            commands::vault::by_tag,
            commands::vault::by_folder,
            commands::vault::by_domain,
            commands::vault::list_tags,
            // ---- entry CRUD + clipboard + move ----
            commands::entry::create_entry,
            commands::entry::update_entry,
            commands::entry::get_entry,
            commands::entry::soft_delete_entry,
            commands::entry::restore_entry,
            commands::entry::hard_delete_entry,
            commands::entry::copy_field,
            commands::entry::move_entry,
            commands::entry::set_favorite,
            commands::entry::set_sort_order,
            commands::entry::set_tags,
            // ---- entry history (slice 2.7) ----
            commands::entry::list_history,
            commands::entry::get_history_value,
            commands::entry::copy_history_field,
            commands::entry::restore_history,
            // ---- tag ops ----
            commands::tag::create_tag,
            commands::tag::rename_tag,
            commands::tag::delete_tag,
            // ---- document import/export ----
            commands::document::import_document,
            commands::document::export_document,
            commands::document::import_document_from_path,
            commands::document::export_document_to_path,
            // ---- password + maintenance ----
            commands::password::change_password,
            commands::maintenance::run_maintenance,
            // ---- emergency kit + recovery ----
            commands::emergency_kit::export_emergency_kit,
            commands::emergency_kit::emergency_kit_pdf,
            commands::emergency_kit::write_emergency_kit_pdf,
            commands::recovery::unlock_with_recovery_key,
            // ---- app.db: recent vaults ----
            commands::recent::list_recent_vaults,
            commands::recent::list_recent_vaults_with_status,
            commands::recent::add_recent_vault,
            commands::recent::remove_recent_vault,
            commands::recent::touch_recent_vault,
            commands::recent::touch_recent_vault_on_unlock,
            commands::recent::remove_stale_recent_vaults,
            // ---- app.db: settings + themes ----
            commands::settings::get_app_setting,
            commands::settings::set_app_setting,
            commands::settings::delete_app_setting,
            commands::settings::list_app_settings,
            commands::settings::list_themes,
            commands::settings::get_theme,
            commands::settings::get_active_theme,
            commands::settings::set_active_theme,
            commands::settings::create_custom_theme,
            commands::settings::update_custom_theme,
            commands::settings::duplicate_theme,
            commands::settings::delete_custom_theme,
            // ---- app.db: devices + extension sessions ----
            commands::device::list_known_devices,
            commands::device::upsert_known_device,
            commands::device::delete_known_device,
            commands::device::touch_known_device_last_seen,
            commands::device::list_extension_sessions,
            commands::device::upsert_extension_session,
            commands::device::delete_extension_session,
            commands::device::touch_extension_session_last_active,
        ])
        .run(tauri::generate_context!())
        .expect("tauri runtime error");
}
