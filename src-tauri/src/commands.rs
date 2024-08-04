use diesel::{query_dsl::methods::SelectDsl, ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{clip::Clip, db, preprocessing, schema::files, state::SearchState};

#[tauri::command]
pub fn search_images(query_string: &str, state: tauri::State<SearchState>) -> Vec<String> {
    if query_string.is_empty() {
        return vec![];
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

    let results = SelectDsl::select(files::table
        .filter(files::id.eq_any(search_results_uuids)), files::relative_path)
        .load::<String>(connection).unwrap();

    results
}
