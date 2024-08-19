-- Your SQL goes here
CREATE TABLE base_directories (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    path TEXT NOT NULL,
    UNIQUE(path)
);
-- Create an index on the path field
CREATE INDEX base_directories_path_index ON base_directories(path);

CREATE TABLE files (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    base_directory_id VARCHAR(36) NOT NULL,
    relative_path TEXT NOT NULL,
    UNIQUE(base_directory_id, relative_path),
    FOREIGN KEY (base_directory_id) REFERENCES base_directories(id)
);
-- Create indices to facilitate searching by filenames or directories.
-- This is necessary for e.g. handling filesystem events where we know paths,
-- but not necessarily the IDs.
-- Create an index on the base_directory_id field
CREATE INDEX files_base_directory_id_index ON files(base_directory_id);
-- Create an index on the base directory id and relative path fields.
-- Useful in combination with base_directories_path_index to get the base directory ID.
CREATE INDEX files_base_directory_id_relative_path_index ON files(base_directory_id, relative_path);