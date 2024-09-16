CREATE TABLE watched_directories (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    filepath TEXT NOT NULL,
    UNIQUE(filepath)
);
CREATE INDEX watched_directories_filepath_index ON watched_directories(filepath);

CREATE TABLE files (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    filepath TEXT NOT NULL,
    watched_directory_id VARCHAR(36),
    FOREIGN KEY (watched_directory_id) REFERENCES watched_directories(id)
    UNIQUE(filepath)
);

CREATE INDEX files_filepath_index ON files(filepath);