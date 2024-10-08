use std::sync::Mutex;

use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, SqliteConnection};
use instant_clip_tokenizer;
use notify_debouncer_full::{notify::RecommendedWatcher, Debouncer, FileIdMap};

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

pub struct InnerConnectionPoolState
{
    pub pool: Pool<ConnectionManager<SqliteConnection>>,
}

pub struct ConnectionPoolState(pub Mutex<InnerConnectionPoolState>);

impl ConnectionPoolState
{
    pub fn get_connection(&self) -> anyhow::Result<PooledConnection<ConnectionManager<SqliteConnection>>>
    {
        Ok(self.0.lock().unwrap().pool.get()?)
    }
}

pub struct InnerClipTokenizerState
{
    pub tokenizer: instant_clip_tokenizer::Tokenizer,
}

pub struct ClipTokenizerState(pub Mutex<InnerClipTokenizerState>);

pub struct FsInnerWatcherState
{
    /// Maps from a path to a debouncer for that path.
    pub watchers: std::collections::HashMap<String, Debouncer<RecommendedWatcher, FileIdMap>>,
}

pub struct FsWatcherState(pub Mutex<FsInnerWatcherState>);