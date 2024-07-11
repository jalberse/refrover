#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use app::models::*;
use diesel::prelude::*;

mod db;

#[tauri::command]
async fn on_button_clicked() -> String {
    // TODO Test out reading in some data and returning it here.
    //      Just go populate it manually for now.
    //      Decide on the type of the returned data. What's a good generic approach?
    //      I'm sure there's some standard, is it just JSON?

    // Loose roadmap:
    // TODO Once we have that, just go right in and decide on our schema for tagging.
    // TODO Then make the frontend work. That's a good milestone - hierarchical tagging
    //      and image display.
    // TODO Then make the returned results useful. Let us copy it onto clipboard, and
    //      drag/paste them into PureRef and other software. This is an MVP.
    // TODO Then we can improve things. Key feature - automatic tagging and fuzzy search
    //      with AI.
    // TODO Then let's set up the website and start selling, before I get into all the other improvements.

    use app::schema::posts::dsl::*;
    let connection = &mut db::establish_db_connection();
    let results = posts
        .filter(published.eq(true))
        .limit(5)
        .select(Post::as_select())
        .load(connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in &results {
        println!("{}", post.title);
        println!("-----------\n");
        println!("{}", post.body);
    }

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
