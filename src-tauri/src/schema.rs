// @generated automatically by Diesel CLI.

diesel::table! {
    base_directories (id) {
        id -> Text,
        path -> Text,
    }
}

diesel::table! {
    file_tags (file_id, tag_id) {
        file_id -> Text,
        tag_id -> Text,
    }
}

diesel::table! {
    files (id) {
        id -> Text,
        base_directory_id -> Text,
        relative_path -> Text,
    }
}

diesel::table! {
    image_features_vit_l_14_336_px (id) {
        id -> Text,
        feature_vector -> Binary,
    }
}

diesel::table! {
    tag_edges (id) {
        id -> Text,
        entry_edge_id -> Text,
        direct_edge_id -> Text,
        exit_edge_id -> Text,
        start_vertex_id -> Text,
        end_vertex_id -> Text,
        hops -> Integer,
        source_id -> Text,
    }
}

diesel::table! {
    tags (id) {
        id -> Text,
        name -> Text,
    }
}

diesel::joinable!(file_tags -> files (file_id));
diesel::joinable!(file_tags -> tags (tag_id));
diesel::joinable!(files -> base_directories (base_directory_id));
diesel::joinable!(image_features_vit_l_14_336_px -> files (id));

diesel::allow_tables_to_appear_in_same_query!(
    base_directories,
    file_tags,
    files,
    image_features_vit_l_14_336_px,
    tag_edges,
    tags,
);
