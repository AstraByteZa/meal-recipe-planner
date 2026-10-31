// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod commands;
mod db;
mod error;
mod keep_awake;
mod models;
mod repo;
mod scrape;
mod sync;

use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(db::init(app.handle()))?;
            let sync = sync::SyncHandle::default();
            app.manage(pool.clone());
            app.manage(sync.clone());
            app.manage(keep_awake::KeepAwakeLock::default());
            sync::worker::spawn(app.handle().clone(), pool, sync);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::recipes_list,
            commands::recipe_get,
            commands::recipe_create,
            commands::recipe_update,
            commands::recipe_delete,
            commands::scrape_recipe_url,
            commands::tracks_list,
            commands::tracks_save,
            commands::schedule_week,
            commands::schedule_set_entry,
            commands::schedule_clear_entry,
            commands::settings_get,
            commands::settings_save,
            commands::settings_complete_onboarding,
            commands::config_list,
            commands::config_set,
            commands::keep_awake_set,
            commands::sync_status,
            commands::sync_register,
            commands::sync_login,
            commands::sync_logout,
            commands::sync_now,
            commands::family_create,
            commands::family_join,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
