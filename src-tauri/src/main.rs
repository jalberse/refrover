#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::time::{SystemTime, UNIX_EPOCH};

mod db;

#[tauri::command]
fn on_button_clicked() -> String {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    format!("on_button_clicked called from Rust! (timestamp: {since_the_epoch}ms)")
}

fn main() {
    // https://blog.moonguard.dev/how-to-use-local-sqlite-database-with-tauri
    //  This is the most straightforward to me - DB is strictly on Rust side,
    //  has migrations, Diesel is cool... yea sure whatever.
    //  https://github.com/diesel-rs/diesel/tree/master/examples/sqlite
    //    Use this while referencing the Diesel docs
    // https://github.com/RandomEngy/tauri-sqlite
    //   Another approach, but migrations unclear.
    // https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/sql
    //  Official Tauri plugin. Build on sqlx rust crate. Queries in TS though??
    //  I don't see a way to configure to get DB set up on Rust side...
    //  Possibly we could add this later, too.

    // TODO Okay, going with Diesel article/approach. Makes the most sense to me.
    // TODO Need to define migrationns (specifically, the first one to define our initial schema).
    //      Do that - start with just "set up a database and display data" and we can create
    //      our real schema later.
    // TODO Ugh, hooking up SQLite and Diesel is annoying. Just gotta do it though...

    tauri::Builder::default()
        .setup(|_app| {
            db::init();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![on_button_clicked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
