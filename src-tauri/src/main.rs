#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use app::{db, queries::query_files_with_tag};

#[tauri::command]
async fn on_button_clicked() -> String {
    let connection = &mut db::establish_db_connection();

    let results = query_files_with_tag(2, connection);

    serde_json::to_string(&results).unwrap()
}

// TODO Thinking from the consumer first, I think we'd want the API to be something like
//    text input -> ranked results.
//  The user wants to be able to just type a search and get the relevant stuff.
//  At first, that can be a simple:
//     decompose the search into tags with boolean operations.
//  Later, we can use different methods - like an AI search, or
//     that searches "fuzzy" tags generated with an AI in a separate table
//     of speculative labels (that we can provide the UI for as "Suggested Labels")
//     which can be promoted into the regular tag table as confirmed by a human.
//    Those can be additional options that are next to the search bar.
//     Other stuff (like tags highlighted in hierarchy, or another visual tag filter)
//     could also impact the final query for fetching.
//    We could even get really fun and have something like a color picker,
//     and use if images include that color predominantly as part of the ranking.

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
