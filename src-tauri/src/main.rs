#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Mutex;

use app::ann::HnswElement;
use app::ann::HnswSearch;
use app::clip::Clip;
use app::db;
use app::models::ImageFeatureVitL14336Px;
use app::preprocessing;
use app::schema::files;
use app::schema::image_features_vit_l_14_336_px;
use diesel::query_dsl::methods::SelectDsl;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::SelectableHelper;
use tauri::App;
use tauri::Manager;
use uuid::Uuid;

#[tauri::command]
async fn on_button_clicked() -> String {
    let connection = &mut db::get_db_connection();

    // TODO I'd need to check the DB to see what the ID is, lol.
    //   Or I can make a query to query the ID by the name, which I might want anyways.
    // let results = query_files_with_tag(2, connection);
    // serde_json::to_string(&results).unwrap()

    "Hello from Rust!".to_string()
}

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

struct SearchState<'a>
{
    hnsw: HnswSearch<'a>,
}

fn main() {
    tauri::Builder::default()
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
            //   Then update get_db_connection() to use that instead.
            // https://docs.rs/diesel/latest/diesel/r2d2/index.html

            app.manage(Mutex::new(
                SearchState
                { 
                    hnsw: HnswSearch::new() 
                }));

            populate_hnsw(app);
            // test_hnsw_with_query(app);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![on_button_clicked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}

fn populate_hnsw(app: &mut App)
{
    let connection = &mut db::get_db_connection();
    
    let results = SelectDsl::select(image_features_vit_l_14_336_px::table, ImageFeatureVitL14336Px::as_select())
        .load::<ImageFeatureVitL14336Px>(connection).unwrap();

    // Create the HnswElements
    let hnsw_elements: Vec<HnswElement> = results.iter().map(|x| HnswElement { feature_vector: bincode::deserialize(&x.feature_vector[..]).unwrap(), id: Uuid::parse_str(&x.id).unwrap() }).collect();

    // Get the HnswSearch from the app's SearchState
    let state = app.state::<Mutex<SearchState>>();
    let mut state = state.lock().unwrap();

    // Add the elements to the Hnsw
    state.hnsw.insert_slice(hnsw_elements);
}

// TODO We'll delete this, but keeping it for dev (we'll grab snippets for a command later
//    where query gets sourced from the search bar)
//  Also including this causes the app to crash (ie continually restart)
//    but it does write the query out lol, so I think the error is in the file stuff.
//    Won't investigate much now because we'll delete this soon anyways. Just move onto adding a command
//     to do similar and return the results to the front-end instead, and hook it up
//     to the search bar.
//  Actually I think it's funnier than that: it restarts because tauri
//    is listening for new files in src-tauri/ and rebuilding when it finds it.
//    So we're not actually crashing. That's really funny and I would have kms
//    if I actually had to solve that.
fn test_hnsw_with_query(app: &mut App)
{
    let connection = &mut db::get_db_connection();

    let state = app.state::<Mutex<SearchState>>();
    let mut state = state.lock().unwrap();
    let hnsw = &mut state.hnsw;

    let query_string = "Tutorial";
    let query = preprocessing::tokenize(query_string);

    let clip = Clip::new().unwrap();
    let query_vector = clip.encode_text(query).unwrap();

    // Convert the query vector to a slice
    let query_vector_slice = query_vector.as_slice().unwrap();

    // Search the Hnsw
    let search_results = hnsw.search(query_vector_slice, 10, 400);

    println!("Search results: {:?}", search_results);

    // Get the UUIDs as strings
    let search_results_uuids: Vec<String> = search_results.iter().map(|x| x.0.to_string()).collect();

    // Get the filenames of the search results from the database using the UUID to select from the files table
    let results = SelectDsl::select(files::table
        .filter(files::id.eq_any(search_results_uuids)), files::relative_path)
        .load::<String>(connection).unwrap();

    use std::fs::File;
    use std::io::Write;
    
    // Since Tauri has an issue with println!() in release, we'll write the results to a file instead
    // (there is a workaround, but I haven't looked closely at it yet)

    // Open a file for writing
    let mut file = File::create("search_results.txt").expect("Failed to create file");
    
    // Write the search results to the file
    writeln!(file, "Search results filenames: {:?}", results).expect("Failed to write to file");
    
    // Close the file
    file.flush().expect("Failed to flush file");
}