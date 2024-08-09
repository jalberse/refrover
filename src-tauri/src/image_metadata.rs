use serde::{Deserialize, Serialize};

/// The size of an image, in pixels.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageSize
{
    pub width: u32,
    pub height: u32,
}

// TODO I'm just going to fork imghdr, add a serde feature, and use that instead of this. I'll also submit a pull request.

/// The metadata for a file; currently, image files.
/// This may include e.g. EXIF metadata, but also metadata from RefRover such as the file ID.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
}