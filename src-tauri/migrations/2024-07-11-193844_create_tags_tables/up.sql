-- Table for storing tags
CREATE TABLE tags (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
);
CREATE INDEX tags_name_index ON tags(name);

-- The tag relationship can be modelled as a directed acyclic graph.
-- Tags can have multiple parent tags, but no cycles are allowed.
-- We follow the model laid out in this article for representing DAGs in SQL:
-- https://www.codeproject.com/Articles/22824/A-Model-to-Represent-Directed-Acyclic-Graphs-DAG-o
create TABLE tag_edges (
    id VARCHAR(36) NOT NULL,
    -- The ID of the incoming edge to the start vertex that is the creation reason for this implied edge; direct edges contain the same value as the Id column
    entry_edge_id VARCHAR(36) NOT NULL,
    -- The ID of the direct edge that caused the creation of this implied edge; direct edges contain the same value as the Id column
    direct_edge_id VARCHAR(36) NOT NULL,
    -- The ID of the outgoing edge from the end vertex that is the creation reason for this implied edge; direct edges contain the same value as the Id column
    exit_edge_id VARCHAR(36) NOT NULL,
    -- The ID of the start vertex (36 is the length of a UUID in string form, if you wonder why)
    start_vertex_id VARCHAR(36) NOT NULL,
    -- The ID of the end vertex
    end_vertex_id VARCHAR(36) NOT NULL,
    -- Indicates how many vertex hops are necessary for the path; it is zero for direct edges
    hops INTEGER NOT NULL,
    -- A column to indicate the context in which the graph is created; useful if we have more than one DAG to be represented within the same application
    -- CAUTION: You need to make sure that the IDs of vertices from different sources never clash; the best is probably use of UUIDs
    source_id VARCHAR(36) NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (entry_edge_id) REFERENCES tag_edges(id),
    FOREIGN KEY (direct_edge_id) REFERENCES tag_edges(id),
    FOREIGN KEY (exit_edge_id) REFERENCES tag_edges(id),
    FOREIGN KEY (start_vertex_id) REFERENCES tags(id),
    FOREIGN KEY (end_vertex_id) REFERENCES tags(id),
    UNIQUE(start_vertex_id, end_vertex_id, source_id)
);

-- Table for associating files with tags
CREATE TABLE file_tags (
    file_id VARCHAR(36) NOT NULL,
    tag_id VARCHAR(36) NOT NULL,
    PRIMARY KEY (file_id, tag_id),
    FOREIGN KEY (file_id) REFERENCES files(id),
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);
-- Useful for getting all tags for a file
CREATE INDEX file_tags_file_id_index ON file_tags(file_id);
