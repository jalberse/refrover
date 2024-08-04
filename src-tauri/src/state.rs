use std::sync::Mutex;

use crate::ann::HnswSearch;

pub struct InnerSearchState<'a>
{
    pub hnsw: HnswSearch<'a>,
}

pub struct SearchState<'a>(pub Mutex<InnerSearchState<'a>>);