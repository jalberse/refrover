-- Your SQL goes here
CREATE TABLE base_directories (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    path TEXT NOT NULL,
    UNIQUE(path)
);

CREATE TABLE files (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    base_directory_id VARCHAR(36) NOT NULL,
    relative_path TEXT NOT NULL,
    UNIQUE(base_directory_id, relative_path),
    FOREIGN KEY (base_directory_id) REFERENCES base_directories(id)
);