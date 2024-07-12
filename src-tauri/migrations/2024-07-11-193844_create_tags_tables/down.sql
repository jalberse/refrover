-- This file should undo anything in `up.sql`
DROP INDEX tags_name_index;
DROP TABLE tags;
DROP TABLE tag_relationships;
DROP TABLE file_tags;