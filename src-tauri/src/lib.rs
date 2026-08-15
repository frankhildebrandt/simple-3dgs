mod app_config;
mod archive;
mod brush;
mod colmap;
mod colmap_pose;
mod commands;
mod error;
mod ffmpeg;
mod geo;
mod html_export;
mod keyframes;
mod pipeline;
mod preset;
mod project;
mod settings;
mod sidecar;
mod train_log;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .setup(|app| {
            // First Documents/archive touch must be on the main thread or macOS
            // TCC returns EPERM without a prompt (IPC commands run on a worker).
            if let Err(err) = commands::load_config(app.handle()) {
                eprintln!("archive access: {err}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_pipeline,
            commands::cancel_pipeline,
            commands::get_config,
            commands::save_config,
            commands::list_archive,
            commands::get_archive,
            commands::rename_archive,
            commands::delete_archive,
            commands::set_archive_poster,
            commands::import_3dgs,
            commands::export_3dgs,
            commands::export_html,
            commands::spz_cache_fresh,
            commands::cache_archive_spz,
            commands::export_spz,
            commands::read_splat_file,
            commands::drop_archive_ply,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
