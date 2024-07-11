-- Your SQL goes here
CREATE TABLE base_directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    path TEXT NOT NULL,
    UNIQUE(path)
);

CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    base_directory_id INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    UNIQUE(base_directory_id, relative_path),
    FOREIGN KEY (base_directory_id) REFERENCES base_directories(id) ON DELETE CASCADE
);