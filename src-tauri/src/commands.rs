use std::path::PathBuf;

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{clip::Clip, db, preprocessing, state::SearchState};

use crate::{junk_drawer, schema};

use image;

// TODO talk about why we return  base64 encoding (can't read files from the front-end for security reasons, see this
//      contentious issue: https://github.com/tauri-apps/tauri/issues/3591
// TODO If the distance is large enough, we should not include it in the results.
//   i.e. we need to filter the search_results on the distance to pass some constant threshold (tweaked by us)
#[tauri::command]
pub async fn search_images<'a>(query_string: &str, state: tauri::State<'_, SearchState<'a>>) -> Result<Vec<String>, String> {
    if query_string.is_empty() {
        return Ok(vec![]);
    }

    let connection = &mut db::get_db_connection();

    let mut hnsw_search = state.0.lock().unwrap();
    let hnsw = &mut hnsw_search.hnsw;

    let query = preprocessing::tokenize(query_string);

    // TODO We should also have CLIP in the state, since we'll only never need one instance.
    let clip = Clip::new().unwrap();
    let query_vector = clip.encode_text(query).unwrap();

    let query_vector_slice = query_vector.as_slice().unwrap();

    let search_results = hnsw.search(query_vector_slice, 10, 400);

    let search_results_uuids: Vec<String> = search_results.iter().map(|x| x.0.to_string()).collect();

    use schema::files;
    use schema::base_directories;

    // Given those UUIDs, fetch the matching files from the files table,
    //  and then get the full path from the base_directories table.
    let all_files: Vec<(String, String, String)> = base_directories::table.inner_join(files::table)
        .filter(files::id.eq_any(&search_results_uuids))
        .select((schema::files::dsl::id, base_directories::path, files::relative_path))
        .load::<(String, String, String)>(connection).unwrap();

    // Combine the base directories and relative paths
    let results: Vec<String> = all_files.iter().map(|(_, base_dir, rel_path)| {
        PathBuf::from(base_dir).join(rel_path)
    }).map(|x| x.to_str().unwrap().to_string()).collect();

    // Read the files and get a base64 encoding of the image.
    // We can return that to the front-end to display the images.
    // 
    let results: Vec<String> = results.iter().map(|x| {
        let img = image::open(x).unwrap();
        junk_drawer::image_to_base64(&img)
    }).collect();

    Ok(results)
}
