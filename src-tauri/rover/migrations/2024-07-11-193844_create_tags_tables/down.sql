-- This file should undo anything in `up.sql`
DROP INDEX tags_name_index;
DROP INDEX file_tags_file_id_index;
DROP TABLE tags;
DROP TABLE tag_edges;
DROP TABLE file_tags;