// TODO Ensure that the field names match too.
// Should be kept in synch with the Rust FileMetadata struct.
interface FileMetadata {
  filename: string
  filepath: string
  dateCreated: string
  dateModified: string
  dimensions: {
    width: number
    height: number
  }
  fileSize: number
  // ...
}

export default FileMetadata
