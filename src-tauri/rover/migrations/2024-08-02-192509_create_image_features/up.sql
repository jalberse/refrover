-- Stores the feature vector of the image according to th ViT-L/14@335px model.
-- The blob should be serialized/deserialized using bincode.
-- We do not contain versioning information; if the feature space changes,
-- a new table should be used, and images can be transfered to a new table.
-- It would be unweildy to return variety of feature representations.
--
-- The referenced file is expected to be an image file, but this is not enforced within the DB.
CREATE TABLE image_features_vit_l_14_336_px (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    feature_vector BLOB NOT NULL,
    FOREIGN KEY (id) REFERENCES files(id)
);