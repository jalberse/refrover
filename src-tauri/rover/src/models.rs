use diesel::prelude::*;
use diesel::sql_types::Integer;
use serde::Serialize;
use time;

use crate::uuid::UUID;

#[derive(QueryableByName)]
pub struct RowsAffected {
    #[sql_type = "Integer"]
    pub rows_affected: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::file_tags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct FileTags {
    pub file_id: UUID,
    pub tag_id: UUID,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::file_tags)]
pub struct NewFileTag {
    pub file_id: UUID,
    pub tag_id: UUID,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct File {
    pub id: UUID,
    pub filepath: String,
    pub watched_directory_id: Option<UUID>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::files)]
pub struct NewFile {
    pub id: UUID,
    pub filepath: String,
    pub watched_directory_id: Option<UUID>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::watched_directories)]
pub struct WatchedDirectory {
    pub id: UUID,
    pub filepath: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::watched_directories)]
pub struct NewWatchedDirectory<'a> {
    pub id: UUID,
    pub filepath: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tag_edges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct TagEdge {
    pub id: UUID,
    pub entry_edge_id: UUID,
    pub direct_edge_id: UUID,
    pub exit_edge_id: UUID,
    pub start_vertex_id: UUID,
    pub end_vertex_id: UUID,
    pub hops: i32,
    pub source_id: UUID,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tag_edges)]
pub struct NewTagEdge {
    pub id: UUID,
    pub entry_edge_id: UUID,
    pub direct_edge_id: UUID,
    pub exit_edge_id: UUID,
    pub start_vertex_id: UUID,
    pub end_vertex_id: UUID,
    pub hops: i32,
    pub source_id: UUID,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct Tags {
    pub id: UUID,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tags)]
pub struct NewTag<'a> {
    pub id: UUID,
    pub name: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::image_features_vit_l_14_336_px)]
pub struct NewImageFeaturesVitL14336Px<'a> {
    pub id: String,
    pub feature_vector: &'a [u8],
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::image_features_vit_l_14_336_px)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct ImageFeatureVitL14336Px {
    pub id: UUID,
    pub feature_vector: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::failed_encodings)]
pub struct NewFailedEncoding {
    pub id: UUID,
    pub error: String,
    // When timestamp is None, the current time (the SQL default) is used.\
    // https://docs.rs/diesel/latest/diesel/fn.insert_into.html#inserting-default-value-for-a-column
    pub failed_at: Option<time::PrimitiveDateTime>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::thumbnails)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct Thumbnail {
    pub id: UUID,
    pub file_id: UUID,
    pub path: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::thumbnails)]
pub struct NewThumbnail<'a> {
    pub id: UUID,
    pub file_id: UUID,
    pub path: &'a str,
}