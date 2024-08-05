#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Mutex;

use app::ann;
use app::ann::HnswSearch;
use app::clip::Clip;
use app::db;
use app::state::ClipState;
use app::state::InnerClipState;
use app::state::InnerSearchState;
use app::state::SearchState;
use tauri::Manager;

// TODO: How do we want to handle new files that are added to watched dirs?
// We need a FileSystemWatcher likely.
//    If we add a file to a watched directory outside our process, we want to add it to the files table.
//    If we are inserting on the files table within the app anyways,
//     it would be cleaner to simply launch an async task at that time
//     to encode the new files and add them to the features table + Hnsw struct.
//    I imagine we'll want notify for that: https://github.com/notify-rs/notify?tab=readme-ov-file
// This can probably be on hold for now. Make a JIRA ticket for it.
//    For now, we can work on the front-end and worry about more "official" paths
//    for adding new files to our system.
// So there should be an "official" pipeline for adding new files to our system. ("ingest"?).
//    Our FileSystem watcher will use it when it sees new files, and we'll
//    use it from the frontend when we know we are adding new files via a more
//    "official" path (like a button in the UI).

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_persisted_scope::init())
        .manage(
            ClipState(
                    Mutex::new(InnerClipState { clip: Clip::new().unwrap() })
                )
            )
        .manage(
            SearchState(
                    Mutex::new(InnerSearchState { hnsw: HnswSearch::new() })
                )
            )
        .setup(|app| {
            db::init();

            // TODO Initialize our KNN index here, loading it from the DB using a new fn.
            // Uh, possibly a lazy-initialized hnsw (so it's globally avail)
            // and then here, load all the feature data and insert.
            //  Hopefully insertion does not take so long that it's a problem every time we start up?
            //  Well, maybe we can serialize the HNSW data structure periodically and just load it on launch?

            // https://v2.tauri.app/develop/state-management/
            // See this for how we'll manage state. The KNN can live in there behind a mutex.
            // We can initialize it here and then store it in the state. Possibly ensure it's heap-allocated.

            // TODO I think that we want to enable the r2d2 feature and also
            //   maintain a connection pool in the app state.
            //   Then update db::get_db_connection() to use that instead.
            // https://docs.rs/diesel/latest/diesel/r2d2/index.html

            // TODO Remove this, just doing for now...
            tauri::scope::FsScope::allow_directory(&app.fs_scope(), "D:\\vizlib_photos", true).expect("Failed to allow access");

            ann::populate_hnsw(app);
            // test_hnsw_with_query(app);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app::commands::search_images])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
