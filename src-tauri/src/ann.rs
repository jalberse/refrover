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

// TODO Rather than a Lazy one like I thought we might have, instead start by defining
//    some hnsw_search struct that wraps everything. We can either lazy initialize that or
//    initialize it in Setup and keep it around in some AppState struct or something.

// Actually this is fine I guess. 
//     //  reading data
//     let anndata = AnnBenchmarkData::new(fname).unwrap();
//     let nb_elem = anndata.train_data.len();
//     let max_nb_connection = 24;
//     let nb_layer = 16.min((nb_elem as f32).ln().trunc() as usize);
//     let ef_c = 400;
//     // allocating network
//     let mut hnsw =  Hnsw::<f32, DistL2>::new(max_nb_connection, nb_elem, nb_layer, ef_c, DistL2{});
//     hnsw.set_extend_candidates(false);
//     // parallel insertion of train data
//     let data_for_par_insertion = anndata.train_data.iter().map( |x| (&x.0, x.1)).collect();
//     hnsw.parallel_insert(&data_for_par_insertion);
//     //
//     hnsw.dump_layer_info();
//     //  Now the bench with 10 neighbours
//     let mut knn_neighbours_for_tests = Vec::<Vec<Neighbour>>::with_capacity(nb_elem);
//     hnsw.set_searching_mode(true);
//     let knbn = 10;
//     let ef_c = max_nb_connection;
//     // search 10 nearest neighbours for test data
//     knn_neighbours_for_tests = hnsw.parallel_search(&anndata.test_data, knbn, ef_c);
//     ....

use hnsw_rs::{hnsw::Hnsw, prelude::DistCosine};
use rustc_hash::FxHashMap;
use uuid::Uuid;

const DEFAULT_MAX_NB_CONNECTION: usize = 24;
const DEFAULT_NB_LAYER: usize = 16;
const DEFAULT_EF_C: usize = 400;
// TODO - Verify that hnsw_rs grows this correctly by setting it to a lower value.
/// The default max number of elements in the HNSW index on creation.
/// This can grow/shink dynamically as elements are added/removed.
const DEFAULT_MAX_ELEMS: usize = 1000;

#[derive(Debug, Clone)]
pub struct HnswElement<'a> {
    pub feature_vector: &'a [f32],
    pub id: Uuid,
}

pub struct HnswSearch<'a> {
    hnsw: Hnsw<'a, f32, DistCosine>,
    max_nb_connection: usize,
    nb_layer: usize,
    ef_c: usize,
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
        HnswSearch { hnsw, max_nb_connection, nb_layer, ef_c, hnsw_id_to_file_id_map, current_id }
    }

    // TODO also add a method for parallel insertion with hnsw.parallel_insert_slice().
    //      however, this is only efficient if the number of elements is large
    //     (greater than 1000 * numThreads). For now, we likely don't need that.
    // TODO Also consider adding a parallel search method, though it isn't currently needed.

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
            .map(|(elem, id)| (elem.feature_vector, *id))
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
        let results: Vec<(Uuid, f32)> = knn_neighbours
            .iter()
            .map(|n| -> (Uuid, f32)
            {
                (self.hnsw_id_to_file_id_map[&n.d_id], n.distance)
            })
            .collect();
        results
    }

    // TODO We need to ensure that all vectors are L2 normalized before insertion or query.
    //   Probably store them as such in the DB.
    //   The reason is so that the cosine distance is equivalent to the dot product, and cheaper.

    // TODO Insertion and search functions
    //   We need to store the UUID with the feature vectors that get stored so we can actually
    //   go fetch the document.
    //   The usize param in the insert function from the example code should be replaced by our UUID.
    //   And rather than a Tuple, we should have the vector + UUID. I suppose we could just
    //   So make a new type that's similar to ImageFeatureVitL14336Px but which uses the UUID
    //   rather than String (we need to use String for SQLITE, but don't want that here).
    //   We can have from/into impls for both.
}