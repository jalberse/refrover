-- Your SQL goes here
-- TODO - We want to make the failed_encodings table. Just track IDs and the time they failed.
--   Don't include the path - that is breaking the normal form of the database.

CREATE TABLE failed_encodings (
  id VARCHAR(36) PRIMARY KEY NOT NULL,
  error TEXT NOT NULL,
  failed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (id) REFERENCES files(id)
);