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
use app::state::ConnectionPoolState;
use app::state::InnerClipState;
use app::state::InnerConnectionPoolState;
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

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_persisted_scope::init())
        .manage(
            ConnectionPoolState(
                    Mutex::new(InnerConnectionPoolState { pool: db::get_connection_pool()? })
                )
        )
        .manage(
            ClipState(
                    Mutex::new(InnerClipState { clip: Clip::new()? })
                )
            )
        .manage(
            SearchState(
                    Mutex::new(InnerSearchState { hnsw: HnswSearch::new() })
                )
            )
        .setup(|app| {

            // Use e.g. `pnpm tauri dev --release -- -- -p` to pass arguments.
            // Multiple `--` are needed to pass arguments to the binary.
            let populate_dummy_data = match app.get_cli_matches() {
                Ok(matches) => 
                {
                    matches.args["populate-dummy-data"].value.as_bool().ok_or(anyhow::anyhow!("Failed to get value for populate-dummy-data"))?
                },
                Err(_) => 
                {
                    false
                },
            };

            let pool_state = app.state::<ConnectionPoolState>();

            db::init(&pool_state, populate_dummy_data)?;

            // TODO Remove this, just doing for now... Will need to replace with our watched directories thing.
            tauri::scope::FsScope::allow_directory(&app.fs_scope(), "D:\\refrover_photos", true)?;

            // We rebuild every time the app launches; it is fast enough, and it handles the fact that
            // we can't remove elements from the HNSW index.
            ann::populate_hnsw(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::search_images,
            app::commands::fetch_thumbnails,
            app::commands::fetch_metadata,
            ])
        .run(tauri::generate_context!())?;

    Ok(())
}
