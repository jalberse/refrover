-- Table for storing tags
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE(name)
);

-- Table for storing the hierarchical relationships between tags
CREATE TABLE tag_relationships (
    parent_tag_id INTEGER NOT NULL,
    child_tag_id INTEGER NOT NULL,
    PRIMARY KEY (parent_tag_id, child_tag_id),
    FOREIGN KEY (parent_tag_id) REFERENCES tags(id),
    FOREIGN KEY (child_tag_id) REFERENCES tags(id)
);

-- Table for associating files with tags
CREATE TABLE file_tags (
    file_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (file_id, tag_id),
    FOREIGN KEY (file_id) REFERENCES files(id),
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);