// Should be kept in synch with the Rust FileMetadata struct.
interface FileMetadata {
  file_id: string
  filename: string
  image_type: string | null
  size: {
    width: number
    height: number
  } | null
  date_created: string | null
  date_modified: string | null
}

export default FileMetadata
