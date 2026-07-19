pub mod commands;
pub mod domain;
pub mod error;
pub mod infra;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init());

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    builder
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::verify::verify_single_email,
            commands::verify::verify_batch_emails,
            commands::verify::cancel_batch_verification,
            commands::verify::export_results_to_csv,
            commands::verify::parse_imported_emails,
            commands::history::fetch_history,
            commands::history::count_history,
            commands::history::fetch_distinct_domains,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::dashboard::get_dashboard_stats,
            commands::dashboard::check_network_health,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
