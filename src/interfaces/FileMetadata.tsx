// Should be kept in synch with the Rust FileMetadata struct.
type FileMetadata = {
  file_id: string
  filename: string
  thumbnail_filepath: string
  image_type: string | null
  size: {
    width: number
    height: number
  } | null
  date_created: string | null
  date_modified: string | null
}

export default FileMetadata
