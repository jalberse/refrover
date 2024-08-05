use std::path::PathBuf;

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;

use crate::state::ClipState;
use crate::{db, preprocessing, state::SearchState};

use crate::{junk_drawer, schema};


// TODO Note that this currently returns base64 png encodings of the images.
//      This is because the front-end is not allowed to access arbitrary files on the system
//      (which I think is dumb - see https://github.com/tauri-apps/tauri/issues/3591 contention).
//      But nevertheless, eventually we want to move to displaying *thumbnails* in the vast majority of cases,
//      which we'll be able to store in a whitelisted location for the frontend.
//      So bear in mind this command API will change to accomodate that (returning a struct
//      with data related to thumbnail paths etc most likely, with base64 as some fallback for edge cases
//      such as very wide images that we can't thumbnail effectively).
// TODO If the distance is large enough, we should not include it in the results.
//   i.e. we need to filter the search_results on the distance to pass some constant threshold (tweaked by us)

/// Search for image UUIDs which match a query string according to CLIP encodings.
/// Returns a list of UUIDs of images which match the query.
/// We return the UUIDs so that separate API calls can be made to fetch the metadata
/// and thumbnails; this allows us to display metadata and results more quickly
/// while the thumbnails are still loading/generating.
#[tauri::command]
pub async fn search_images<'a>(
        query_string: &str,
        search_state: tauri::State<'_, SearchState<'a>>,
        clip_state: tauri::State<'_, ClipState>,
    ) -> Result<Vec<String>, String>
{
    if query_string.is_empty() {
        return Ok(vec![]);
    }

    let mut hnsw_search = search_state.0.lock().unwrap();
    let hnsw = &mut hnsw_search.hnsw;

    let query = preprocessing::tokenize(query_string);

    let clip = &mut clip_state.0.lock().unwrap().clip;

    let query_vector = clip.encode_text(query).unwrap();

    let query_vector_slice = query_vector.as_slice().unwrap();

    let search_results = hnsw.search(query_vector_slice, 10, 400);

    let search_results_uuids: Vec<String> = search_results.iter().map(|x| x.0.to_string()).collect();

    Ok(search_results_uuids)
}
