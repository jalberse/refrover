use diesel::prelude::*;
use diesel::sql_types::Integer;
use serde::Serialize;
use time;

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
    pub file_id: String,
    pub tag_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::file_tags)]
pub struct NewFileTag<'a> {
    pub file_id: &'a str,
    pub tag_id: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct File {
    pub id: String,
    pub filepath: String,
    pub watched_directory_id: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::files)]
pub struct NewFile<'a> {
    pub id: &'a str,
    pub filepath: &'a str,
    pub watched_directory_id: Option<&'a str>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::files)]
pub struct NewFileOwned {
    pub id: String,
    pub filepath: String,
    pub watched_directory_id: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::watched_directories)]
pub struct WatchedDirectory {
    pub id: String,
    pub filepath: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::watched_directories)]
pub struct NewWatchedDirectory<'a> {
    pub id: &'a str,
    pub filepath: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tag_edges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct TagEdge {
    pub id: String,
    pub entry_edge_id: String,
    pub direct_edge_id: String,
    pub exit_edge_id: String,
    pub start_vertex_id: String,
    pub end_vertex_id: String,
    pub hops: i32,
    pub source_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tag_edges)]
pub struct NewTagEdge<'a> {
    pub id: &'a str,
    pub entry_edge_id: &'a str,
    pub direct_edge_id: &'a str,
    pub exit_edge_id: &'a str,
    pub start_vertex_id: &'a str,
    pub end_vertex_id: &'a str,
    pub hops: i32,
    pub source_id: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct Tags {
    pub id: String,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tags)]
pub struct NewTag<'a> {
    pub id: &'a str,
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
    pub id: String,
    pub feature_vector: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::failed_encodings)]
pub struct NewFailedEncoding {
    pub id: String,
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
    pub id: String,
    pub file_id: String,
    pub path: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::thumbnails)]
pub struct NewThumbnail<'a> {
    pub id: &'a str,
    pub file_id: &'a str,
    pub path: &'a str,
}