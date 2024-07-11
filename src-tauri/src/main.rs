#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use app::{models::*, schema::base_directories};
use diesel::prelude::*;
 use app::db;

#[tauri::command]
async fn on_button_clicked() -> String {
    // TODO Instead return a list of files that match some criteria.
    // TODO And let's make another button that populates the db with, say, the files in some hardcoded folder (eventually, the user will be able to choose the folder).

    // use app::schema::posts::dsl::*;
    // let connection = &mut db::establish_db_connection();
    // let results = posts
    //     .filter(published.eq(true))
    //     .limit(5)
    //     .select(Post::as_select())
    //     .load(connection)
    //     .expect("Error loading posts");

    // println!("Displaying {} posts", results.len());
    // for post in &results {
    //     println!("{}", post.title);
    //     println!("-----------\n");
    //     println!("{}", post.body);
    // }


    "".to_string()

    // serde_json::to_string(&results).unwrap()
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
