#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use app::{db}

#[tauri::command]
async fn on_button_clicked() -> String {
    let connection = &mut db::establish_db_connection();

    // TODO I'd need to check the DB to see what the ID is, lol.
    //   Or I can make a query to query the ID by the name, which I might want anyways.
    // let results = query_files_with_tag(2, connection);
    // serde_json::to_string(&results).unwrap()

    "Hello from Rust!".to_string()
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            db::init();

            // TODO Initialize our KNN index here, loading it from the DB using a new fn.

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![on_button_clicked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
