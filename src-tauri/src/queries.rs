/// Queries that operate on the database which contain core logic;
/// queries related to the database itself (e.g. to enable foreign keys)
/// are handled in the db module.

use std::path::PathBuf;

use diesel::prelude::*;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl, SqliteConnection};

// Function to query for all files with a given tgag, including parent tags
pub fn query_files_with_tag(tag_id: i32, connection: &mut SqliteConnection) -> Vec<String>
{
    use crate::schema::{file_tags, files, base_directories};

    let tag_ids = find_containing_tags(tag_id, connection);

    let file_ids = file_tags::table
        .filter(file_tags::tag_id.eq_any(tag_ids))
        .select(file_tags::file_id)
        .load::<i32>(connection)
        .expect("Error loading file IDs");

    let files: Vec<(String, String)> = files::table
        .filter(files::id.eq_any(file_ids))
        .inner_join(base_directories::table.on(files::base_directory_id.eq(base_directories::id)))
        .select((base_directories::path, files::relative_path))
        .load::<(String, String)>(connection)
        .expect("Error loading files");

    let files = files.into_iter().map(|(parent_dir, relpath)| -> String {
        let parent_dir = PathBuf::from(parent_dir);
        let relpath = PathBuf::from(relpath);

        let file_path = parent_dir.join(relpath);

        file_path.to_string_lossy().into_owned()
    }).collect();

    files
}

// Returns tag_id and its parents, recursively.
pub fn find_containing_tags(tag_id: i32, connection: &mut SqliteConnection) -> Vec<i32>
{
    use crate::schema::tag_relationships::dsl::*;

    let mut parent_tags = vec![tag_id];
    let mut current_tag_id = Some(tag_id);

    while let Some(tag) = current_tag_id {
        let parent_tag = tag_relationships
            .filter(child_tag_id.eq(tag))
            .select(parent_tag_id)
            .first::<i32>(connection)
            .optional()
            .expect("Error finding parent tag");

        if let Some(parent_tag) = parent_tag {
            parent_tags.push(parent_tag);
            current_tag_id = Some(parent_tag);
        } else {
            current_tag_id = None;
        }
    }

    parent_tags
}