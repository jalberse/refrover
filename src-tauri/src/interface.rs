/// This module contains structs that are returned by the various Tauri commands.
/// They are serialized as JSON objects to be received by the front-end in api.tsx.
/// Their definitions should be kept in sync with the front-end's interfaces/ module types.
/// That INCLUDES the names of the various fields, which become the JSON keys.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Thumbnail
{
    pub uuid: ThumbnailUuid,
    pub file_uuid: FileUuid,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ThumbnailUuid(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FileUuid(pub String);

/// The size of an image, in pixels.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ImageSize
{
    pub width: u32,
    pub height: u32,
}

/// The metadata for a file; currently, image files.
/// This may include e.g. EXIF metadata, but also metadata from RefRover such as the file ID.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FileMetadata
{
    pub file_id: String,
    pub filename: String,
    pub image_type: Option<imghdr::Type>,
    pub size: Option<ImageSize>,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub date_accessed: Option<String>,
    // TODO Other metadata fields such as EXIF information from the camera?
}

#[cfg(test)]
mod tests 
{
    use super::*;

    #[test]
    fn imghdr_type_serialization()
    {
        let image_type = imghdr::Type::Jpeg;
        let serialized = serde_json::to_string(&image_type).unwrap();
        let deserialized: imghdr::Type = serde_json::from_str(&serialized).unwrap();
        assert_eq!(image_type, deserialized);
    }

    #[test]
    fn metadata_serialization()
    {
        let metadata = FileMetadata
        {
            file_id: "1234".to_string(),
            filename: "test.jpg".to_string(),
            image_type: Some(imghdr::Type::Jpeg),
            size: Some(ImageSize { width: 1920, height: 1080 }),
            date_created: Some("2021-01-01".to_string()),
            date_modified: Some("2021-01-02".to_string()),
            date_accessed: Some("2021-01-03".to_string()),
        };
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: FileMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(metadata, deserialized);
    }
}