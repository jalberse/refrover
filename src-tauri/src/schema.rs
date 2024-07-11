// @generated automatically by Diesel CLI.

diesel::table! {
    base_directories (id) {
        id -> Integer,
        path -> Text,
    }
}

diesel::table! {
    file_tags (file_id, tag_id) {
        file_id -> Integer,
        tag_id -> Integer,
    }
}

diesel::table! {
    files (id) {
        id -> Integer,
        base_directory_id -> Integer,
        relative_path -> Text,
    }
}

diesel::table! {
    tag_relationships (parent_tag_id, child_tag_id) {
        parent_tag_id -> Integer,
        child_tag_id -> Integer,
    }
}

diesel::table! {
    tags (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::joinable!(file_tags -> files (file_id));
diesel::joinable!(file_tags -> tags (tag_id));
diesel::joinable!(files -> base_directories (base_directory_id));

diesel::allow_tables_to_appear_in_same_query!(
    base_directories,
    file_tags,
    files,
    tag_relationships,
    tags,
);
