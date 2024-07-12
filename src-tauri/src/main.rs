#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use app::db::query_files_with_tag;
 use app::db;

#[tauri::command]
async fn on_button_clicked() -> String {
    let connection = &mut db::establish_db_connection();

    let results = query_files_with_tag(2, connection);

    serde_json::to_string(&results).unwrap()
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            db::init();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![on_button_clicked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
