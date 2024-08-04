#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use app::db;
use app::ann;
use app::models::ImageFeatureVitL14336Px;
use diesel::query_dsl::methods::SelectDsl;
use diesel::RunQueryDsl;
use diesel::SelectableHelper;

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
            // Uh, possibly a lazy-initialized hnsw (so it's globally avail)
            // and then here, load all the feature data and insert.
            //  Hopefully insertion does not take so long that it's a problem every time we start up?
            //  Well, maybe we can serialize the HNSW data structure periodically and just load it on launch?

            test_hnsw();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![on_button_clicked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn test_hnsw()
{
    let connection = &mut db::establish_db_connection();
    
    // Load image_features_vit_l_14_336_px from the DB
    use app::schema::image_features_vit_l_14_336_px::dsl::*;
    let results = image_features_vit_l_14_336_px
    .select(ImageFeatureVitL14336Px::as_select())
    .load(connection)
    .expect("Error loading image features");

    // Deserialize the feature vectors
    let feature_vectors: Vec<Vec<f32>> = results.iter().map(|x| bincode::deserialize(&x.feature_vector[..]).unwrap()).collect();

    // TODO Lifetime hell. Why does hnsw require a lifetime on the feature vectors? I hate that.
    // Well, start with perhaps HnswElement owning the vec, not a slice. Maybe I'm misinterpreting.

    let mut hnsw = ann::HnswSearch::new();
}