//! Vedge Tauri shell. Thin adapter over `vedge-core` use cases.

pub mod breach;
pub mod commands;
pub mod dto;
pub mod error;
pub mod pdf;
pub mod scheduler;
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
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

        // The single-instance lock is keyed by the app identifier, not the
        // data directory — two instances collide even with distinct
        // `VEDGE_DATA_DIR`s. Skip it in portable/test mode (any non-empty
        // `VEDGE_DATA_DIR`) so a portable copy — or an e2e run, which also
        // needs to relaunch the app for its restart test — doesn't
        // forward-and-exit into a normally-installed instance. See
        // `setup::services::resolve_app_dir`.
        let portable = std::env::var_os("VEDGE_DATA_DIR").is_some_and(|v| !v.is_empty());
        if !portable {
            builder = builder.plugin(tauri_plugin_single_instance::init(|_, _, _| {}));
        }
    }

    builder
        .setup(|app| {
            // `compose` is `!Send` (holds &tauri::App); run it on the
            // current thread via Tauri's async runtime. The error type of
            // `compose` is `Box<dyn Error + Send + Sync>`; tauri's setup
            // hook wants `Box<dyn Error>` — just re-box.
            let state = tauri::async_runtime::block_on(setup::services::compose(app))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            // Grab the registry handles + screen-lock watcher before `manage`
            // consumes `state`, then start the background scheduler: the
            // hard-session-TTL reaper (4.5a) + pending-lock audit queue (4.6a),
            // the OS screen-lock sweep (4.5b), and the 6 h maintenance job (4.6a),
            // all sharing the same `Arc<Mutex<..>>` handles.
            let sessions = state.sessions_handle();
            let pending_locks = state.pending_locks_handle();
            let screen_lock = std::sync::Arc::clone(&state.screen_lock);
            app.manage(state);
            scheduler::spawn(sessions, pending_locks, screen_lock);
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
            // ---- audit log (slice 4.1) ----
            commands::audit::list_audit,
            // ---- entry CRUD + clipboard + move ----
            commands::entry::create_entry,
            commands::entry::update_entry,
            commands::entry::get_entry,
            commands::entry::soft_delete_entry,
            commands::entry::restore_entry,
            commands::entry::hard_delete_entry,
            commands::entry::copy_field,
            commands::clipboard::copy_text,
            commands::entry::move_entry,
            commands::entry::set_favorite,
            commands::entry::set_sort_order,
            commands::entry::set_tags,
            // ---- entry history (slice 2.7) ----
            commands::entry::list_history,
            commands::entry::get_history_value,
            commands::entry::copy_history_field,
            commands::entry::restore_history,
            // ---- TOTP (slice 4.2) ----
            commands::totp::reveal_totp,
            commands::totp::parse_totp_enrolment,
            // ---- password health (slice 4.3) ----
            commands::health::scan_health,
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
            // ---- export entries (slice 5.3a) ----
            commands::export::export_entries,
            // ---- import entries (slice 5.3b) ----
            commands::import::begin_import,
            commands::import::begin_snapshot_import,
            commands::import::commit_import,
            commands::import::cancel_import,
            // ---- backup (slice 5.2) ----
            commands::backup::backup_vault,
            // ---- restore (slice 5.2b) ----
            commands::backup::inspect_backup,
            commands::backup::open_backup,
            commands::backup::replace_vault_from_backup,
            commands::backup::backup_status,
            commands::backup::delete_vault,
            commands::backup::vault_details,
            // ---- snapshots (slice 5.2.1) ----
            commands::snapshot::create_snapshot,
            commands::snapshot::list_snapshots,
            commands::snapshot::delete_snapshot,
            commands::snapshot::revert_to_snapshot,
            commands::snapshot::revert_to_snapshot_in_session,
            // ---- emergency kit + recovery ----
            commands::emergency_kit::export_emergency_kit,
            commands::emergency_kit::emergency_kit_pdf,
            commands::emergency_kit::write_emergency_kit_pdf,
            commands::recovery::unlock_with_recovery_key,
            // ---- biometric unlock (slice 2.8) ----
            commands::biometric::biometric_available,
            commands::biometric::biometric_is_enrolled,
            commands::biometric::biometric_enroll,
            commands::biometric::biometric_unlock,
            commands::biometric::biometric_disable,
            // ---- app.db: recent vaults ----
            commands::registry::list_registered_vaults,
            commands::registry::list_registered_vaults_with_status,
            commands::registry::register_vault,
            commands::registry::deregister_vault,
            commands::registry::touch_registered_vault,
            commands::registry::touch_registered_vault_on_unlock,
            commands::registry::rename_registered_vault,
            commands::registry::remove_stale_registered_vaults,
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
