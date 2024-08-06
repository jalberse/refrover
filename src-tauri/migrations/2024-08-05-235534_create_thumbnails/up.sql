CREATE TABLE thumbnails (
  id VARCHAR(36) PRIMARY KEY NOT NULL,
  -- The file for which this thumbnail is created
  file_id VARCHAR(36) NOT NULL,
  path TEXT NOT NULL,
  FOREIGN KEY (file_id) REFERENCES files(id)
);
CREATE INDEX thumbnails_file_id ON thumbnails(file_id);