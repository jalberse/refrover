// @generated automatically by Diesel CLI.

diesel::table! {
    failed_encodings (id) {
        id -> Text,
        error -> Text,
        failed_at -> Timestamp,
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
        filepath -> Text,
        watched_directory_id -> Nullable<Text>,
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

diesel::table! {
    thumbnails (id) {
        id -> Text,
        file_id -> Text,
        path -> Text,
    }
}

diesel::table! {
    watched_directories (id) {
        id -> Text,
        filepath -> Text,
    }
}

diesel::joinable!(failed_encodings -> files (id));
diesel::joinable!(file_tags -> files (file_id));
diesel::joinable!(file_tags -> tags (tag_id));
diesel::joinable!(files -> watched_directories (watched_directory_id));
diesel::joinable!(image_features_vit_l_14_336_px -> files (id));
diesel::joinable!(thumbnails -> files (file_id));

diesel::allow_tables_to_appear_in_same_query!(
    failed_encodings,
    file_tags,
    files,
    image_features_vit_l_14_336_px,
    tag_edges,
    tags,
    thumbnails,
    watched_directories,
);
