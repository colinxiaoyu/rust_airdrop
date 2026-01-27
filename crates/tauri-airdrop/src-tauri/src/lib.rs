mod commands;
mod daemon_bridge;
mod state;

use state::AppState;
use tauri::Manager;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("tauri_airdrop=debug".parse().unwrap())
                .add_directive("daemon=debug".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 初始化应用状态
            let state = AppState::new(app.handle().clone());
            app.manage(state);

            // 启动 Daemon 后台任务
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                daemon_bridge::run_daemon(app_handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_file,
            commands::list_peers,
            commands::get_device_info,
            commands::get_download_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
