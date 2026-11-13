mod commands;
mod edition_data;
pub mod helper;
mod system_theme;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin registered: it needs to intercept and exit
        // a second launch before anything else runs. Two installers working on
        // the same disk at once must never happen.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            system_theme::get_system_theme,
            commands::environment::get_environment_info,
            commands::requirements::check_requirements,
            commands::requirements::check_internet,
            commands::location::get_location_data,
            commands::location::get_keyboard_layouts,
            commands::location::detect_timezone,
            commands::location::set_live_keyboard,
            commands::disks::list_disks,
            commands::disks::partition_min_size,
            commands::install::start_install,
            commands::install::reboot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soplos Linux Installer");
}
