use std::sync::Mutex;

use crate::{ann::HnswSearch, clip::Clip};

pub struct InnerSearchState<'a>
{
    pub hnsw: HnswSearch<'a>,
}

pub struct SearchState<'a>(pub Mutex<InnerSearchState<'a>>);

pub struct InnerClipState
{
    pub clip: Clip,
}

pub struct ClipState(pub Mutex<InnerClipState>);