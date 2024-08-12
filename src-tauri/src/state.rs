use std::sync::Mutex;

use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, SqliteConnection};

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