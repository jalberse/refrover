use anyhow::{Context, Ok};
/// Approximate nearest neighbor search using the HNSW algorithm.
/// This module is a wrapper around the hnsw_rs crate that provides a more
/// convenient API for our use case, setting defaults and taking care of
/// configuration for the caller, and logging as necessary.
/// 
/// While this is a general-purpose module, it is intended to be used in the
/// searching for image feature vectors that match a given text feature vector
/// from a natural language text query generated via CLIP.encode_text().
/// It can also easily be used for reverse image search by using the output of
/// encode_image() as the search vector instead.

use diesel::{query_dsl::methods::SelectDsl, RunQueryDsl, SelectableHelper};
use hnsw_rs::{hnsw::Hnsw, prelude::DistCosine};
use rustc_hash::FxHashMap;
use tauri::{App, Manager};
use uuid::Uuid;

use crate::{db, models::ImageFeatureVitL14336Px, schema::image_features_vit_l_14_336_px, state::{ConnectionPoolState, SearchState}};

const DEFAULT_MAX_NB_CONNECTION: usize = 100;
const DEFAULT_NB_LAYER: usize = 16;
const DEFAULT_EF_C: usize = 400;
const DEFAULT_MAX_ELEMS: usize = 10000;

#[derive(Debug, Clone)]
pub struct HnswElement {
    pub feature_vector: Vec<f32>,
    pub id: Uuid,
}

// TODO We need to ensure that all vectors are L2 normalized before insertion or query.
    //   Probably store them as such in the DB?
    //   The reason is so that the cosine distance is equivalent to the dot product, and cheaper.

/// Note that HNSW does not support removing points.
/// To resolve this, the HNSW structure is rebuilt on start-up each time,
/// which should be fast enough for our application with just a few tens
/// of thousands of images at most. This does mean that IDs returned by
/// a query may not be valid if the corresponding image has been removed.
/// This may mean that the K nearest neighbors might in fact be fewer than K.
pub struct HnswSearch<'a> {
    hnsw: Hnsw<'a, f32, DistCosine>,
    /// The hnsw crate uses usize for the ID of the elements in the index.
    /// We need to map these to the UUIDs of the documents in the database.
    /// While usize may not be large enough to map 1:1 with UUIDs, we functionally
    /// should never have this issue. We use UUIDs to maintain uniqueness across DBs
    /// in case we want to merge them, but usize is fine for this purpose.
    hnsw_id_to_file_id_map: FxHashMap<usize, Uuid>,
    current_id: usize,
}

impl<'a> HnswSearch<'a>
{
    pub fn new() -> HnswSearch<'a>
    {
        let max_nb_connection = DEFAULT_MAX_NB_CONNECTION;
        let nb_layer = DEFAULT_NB_LAYER;
        let ef_c = DEFAULT_EF_C;
        let nb_elem = DEFAULT_MAX_ELEMS;
        let hnsw_id_to_file_id_map = FxHashMap::default();
        let current_id = 0;
        let mut hnsw = Hnsw::<f32, DistCosine>::new(max_nb_connection, nb_elem, nb_layer, ef_c, DistCosine{});
        // Enabled according to ann-glove25-angular example from hnsw_rs.
        hnsw.set_extend_candidates(true);
        HnswSearch
        { 
            hnsw,
            hnsw_id_to_file_id_map,
            current_id
        }
    }

    pub fn insert_slice(&mut self, data: Vec<HnswElement>)
    {
        // Generate the IDs for the elements
        let ids: Vec<usize> = (self.current_id..self.current_id + data.len()).collect();
        self.current_id += data.len();
        // Map the IDs to the UUIDs
        for (id, elem) in ids.iter().zip(data.iter())
        {
            self.hnsw_id_to_file_id_map.insert(*id, elem.id);
        }
        // Convert the data to the format needed by hnsw_rs
        let data_for_par_insertion: Vec<(&[f32], usize)> = data
            .iter()
            .zip(ids.iter())
            .map(|(elem, id)| (&elem.feature_vector[..], *id))
            .collect();
        // Insert the elements into the HNSW index
        for d in data_for_par_insertion.iter()
        {
            self.hnsw.insert_slice(*d);
        }
    }

    /// Returns a vector of IDs of the knbn nearest neighbors, with the corresponding distances (for eg ranking).
    /// The IDs are the UUIDs of the documents in the database.
    /// The distances are the cosine distances between the query vector and the feature vectors of the neighbors.
    /// 
    /// @param  ef_arg This parameter controls the width of the search in the lowest level,
    /// it must be greater than number of neighbours asked but can be less than ef_construction.
    /// As a rule of thumb could be between the number of neighbours we will ask for (knbn arg in search method) and DEFAULT_MAX_NB_CONNECTION.
    pub fn search(&self, query: &[f32], knbn: usize, ef_arg: usize) -> Vec<(Uuid, f32)>
    {
        let knn_neighbours = self.hnsw.search(query, knbn, ef_arg);
        // Map the IDs to the UUIDs. Neighbor.d_id (short for data_id) corresponds to the usize ID.

        // TODO Filter these results based on some constant, tweak it.
        let results: Vec<(Uuid, f32)> = knn_neighbours
            .iter()
            .map(|n| -> (Uuid, f32)
            {
                (self.hnsw_id_to_file_id_map[&n.d_id], n.distance)
            })
            .collect();
        results
    }
}

/// Populates the HNSW index with the feature vectors from the database, intended for startup.
/// As more images are added to the application during runtime, they should be added to the HNSW index as necessary.
pub fn populate_hnsw(app: &mut App) -> anyhow::Result<()>
{
    let pool_state = app.state::<ConnectionPoolState>();

    let connection = &mut db::get_db_connection(&pool_state)?;
    
    let results = SelectDsl::select(image_features_vit_l_14_336_px::table, ImageFeatureVitL14336Px::as_select())
        .load::<ImageFeatureVitL14336Px>(connection).context("Unable to load image features")?;

    // TODO Work anyhow into this closure, it's a bit annoying. fetch_thumbnails() does basically what we'd need to do;
    //      the iterator implements Into for anyhow::Result, so you just need to specify that type for hnsw_elements.
    //      Closures are the only slightly tricky bit left, so just do it and we can propagate up through commands etc and close the ticket.
    // Create the HnswElements
    let hnsw_elements = results.iter().map(
        |x| Ok(HnswElement 
        {
            feature_vector: bincode::deserialize(&x.feature_vector[..])?,
            id: Uuid::parse_str(&x.id)?,
        })).collect::<anyhow::Result<Vec<HnswElement>>>()?;

    // Get the HnswSearch from the app's SearchState
    let state = app.state::<SearchState>();
    let mut state = state.0.lock().unwrap();

    // Add the elements to the Hnsw
    state.hnsw.insert_slice(hnsw_elements);

    Ok(())
}
