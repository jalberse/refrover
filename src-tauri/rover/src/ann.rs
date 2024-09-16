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

use crate::{db, models::ImageFeatureVitL14336Px, schema::image_features_vit_l_14_336_px, state::{ConnectionPoolState, SearchState}, uuid::UUID};

// The maximum number of links from one point to others.
// Values from 16 to 64 are standard, with higher being more time consuming.
pub const DEFAULT_MAX_NB_CONNECTION: usize = 64;
// The maximum number of layers in graph
// Must be less than or equal to 16.
pub const DEFAULT_NB_LAYER: usize = 16;
// This parameter controls the width of the search for neighbours during insertion.
// Values from 400 to 800 are standard, with higher being more time consuming.
pub const DEFAULT_EF_CONSTRUCTION: usize = 400;
pub const DEFAULT_MAX_ELEMS: usize = 10000;

#[derive(Debug, Clone)]
pub struct HnswElement {
    pub feature_vector: Vec<f32>,
    pub id: UUID,
}

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
    hnsw_id_to_file_id_map: FxHashMap<usize, UUID>,
    current_id: usize,
}

impl<'a> HnswSearch<'a>
{
    pub fn new() -> HnswSearch<'a>
    {
        let max_nb_connection = DEFAULT_MAX_NB_CONNECTION;
        let nb_layer = DEFAULT_NB_LAYER;
        let ef_c = DEFAULT_EF_CONSTRUCTION;
        let nb_elem = DEFAULT_MAX_ELEMS;
        let hnsw_id_to_file_id_map = FxHashMap::default();
        let current_id = 0;
        let mut hnsw = Hnsw::<f32, DistCosine>::new(
            max_nb_connection, 
            nb_elem, 
            nb_layer,
            ef_c,
            DistCosine{}
            );
        // Enabled according to ann-glove25-angular example from hnsw_rs.
        // Angular data may be highly clustered, so we enable this.
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
    /// it MUST be greater than number of neighbours asked (knbn) but CAN be less than DEFAULT_EF_CONSTRUCTION.
    /// As a rule of thumb could be between the number of neighbours we will ask for (knbn arg in search method) and DEFAULT_MAX_NB_CONNECTION.
    /// It does not limit the number of neighbours returned; recall will be lower if ef_arg is lower, but search is slower with high ef_arg.
    /// @param distance_threshold the value that the distance must be less than to be included in the results.
    /// We use the cosine distance for our HNSW search.
    /// Range of cosine distance is from 0 to 2, 0 — identical vectors, 1 — no correlation, 2 — absolutely different.
    /// In practice, due to high-dimensional feature vectors, ~0.79 will be very semantically similar,
    /// and ~0.85 will be very semantically different (this is a rough estimate, check for a given dataset).
    pub fn search(&self, query: &[f32], knbn: usize, ef_arg: usize, distance_threshold: f32) -> Vec<(UUID, f32)>
    {
        let knn_neighbours = self.hnsw.search(query, knbn, ef_arg);
        // Map the IDs to the UUIDs. Neighbor.d_id (short for data_id) corresponds to the usize ID.

        // TODO Filter these results based on some constant, tweak it.
        let results: Vec<(UUID, f32)> = knn_neighbours
            .iter()
            .map(|n| -> (UUID, f32)
            {
                (self.hnsw_id_to_file_id_map[&n.d_id], n.distance)
            })
            .filter(|(_, distance)| *distance < distance_threshold)
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

    // Create the HnswElements
    let hnsw_elements = convert_rows_to_hnsw_elements(&results)?;

    // Get the HnswSearch from the app's SearchState
    let state = app.state::<SearchState>();
    let mut state = state.0.lock().unwrap();

    // Add the elements to the Hnsw
    state.hnsw.insert_slice(hnsw_elements);

    Ok(())
}

pub fn convert_rows_to_hnsw_elements(rows: &[ImageFeatureVitL14336Px]) -> anyhow::Result<Vec<HnswElement>>
{
    Ok(rows.iter().map(
        |x| Ok(HnswElement 
        {
            feature_vector: bincode::deserialize(&x.feature_vector[..])?,
            id: x.id,
        })).collect::<anyhow::Result<Vec<HnswElement>>>()?)
}
