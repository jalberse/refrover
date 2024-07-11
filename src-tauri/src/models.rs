use diesel::prelude::*;
use serde::Serialize;

use crate::schema::base_directories;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::base_directories)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct BaseDirectories {
    pub id: i32,
    pub path: String,
}

// Note that for tables with auto-incrementing IDs, we do not
// pass in the ID explicitly. This is part of the reason for having
// separate structs for Insertable and Queryable models.
#[derive(Insertable)]
#[diesel(table_name = base_directories)]
pub struct NewBaseDirectory<'a> {
    pub path: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::file_tags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct FileTags {
    pub file_id: i32,
    pub tag_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::file_tags)]
pub struct NewFileTag {
    pub file_id: i32,
    pub tag_id: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct Files {
    pub id: i32,
    pub base_directory_id: i32,
    pub relative_path: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::files)]
pub struct NewFile<'a> {
    pub base_directory_id: i32,
    pub relative_path: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tag_relationships)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct TagRelationships {
    pub parent_tag_id: i32,
    pub child_tag_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tag_relationships)]
pub struct NewTagRelationship {
    pub parent_tag_id: i32,
    pub child_tag_id: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[derive(Serialize)]
pub struct Tags {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::tags)]
pub struct NewTag<'a> {
    pub name: &'a str,
}