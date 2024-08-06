use diesel::prelude::*;
use diesel::sql_types::Integer; // Add this line
use serde::Serialize;

use crate::schema::base_directories;

#[derive(QueryableByName)]
pub struct RowsAffected {
    #[sql_type = "Integer"]
    pub rows_affected: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::base_directories)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct BaseDirectories {
    pub id: String,
    pub path: String,
}

// Note that for tables with auto-incrementing IDs, we do not
// pass in the ID explicitly. This is part of the reason for having
// separate structs for Insertable and Queryable models.
#[derive(Insertable)]
#[diesel(table_name = base_directories)]
pub struct NewBaseDirectory<'a> {
    pub id: &'a str,
    pub path: &'a str,
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
pub struct Files {
    pub id: String,
    pub base_directory_id: String,
    pub relative_path: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::files)]
pub struct NewFile<'a> {
    pub id: &'a str,
    pub base_directory_id: &'a str,
    pub relative_path: &'a str,
}

// TODO These don't match the new schema. Check this and all the other ones. They need the other new fields.
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tag_edges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct TagEdges {
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
    pub id: &'a str,
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