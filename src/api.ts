import { invoke } from "@tauri-apps/api/tauri"

// interface FileMetadata {
//     filename: string
//     filepath: string
//     dateCreated: string
//     dateModified: string
//     dimensions: {
//         width: number
//         height: number
//     }
//     // TODO Pick units intentionally. I think we can do bytes and display as needed. Ensure Rust returned matches.
//     fileSize: number
//     // ...
// }

// TODO This will actually probably be just for one UUID, as we'll just call it when we inspect
//      one image.
// Fetches the metadata for the set of files with the given UUIDs.
// Returns a map from UUID to metadata objects.
// export async function fetchMetadata(fileUuids: string[]) {
//     try {
//         // Returns a map from UUID to metadata objects.
//         const result = await invoke<Record<string, FileMetadata>>("fetch_metadata", {
//             fileUuids,
//         })
//         return result
//     } catch (error) {
//         throw new Error("Failed to fetch metadata")
//     }
// }

// Fetches the thumbnails for the set of files with the given UUIDs.
// If no thumbnail is available, it will be generated.
// This may take some time to execute as thumbnails are generated.
// Returns a map from UUID to thumbnail image file names.
// We assume the caller knows the directory storing the thumbnails.
export async function fetchThumbnails(queryString: string) {
  try {
    const result = await invoke<[string][]>("search_images", {
      queryString,
    })

    // The result is a list of file UUIDs.
    // Fetch the thumbnails for these files.
    const fileUuids = result.flat()

    console.log("fileUuids", fileUuids)

    try {
      const result = await invoke<Record<string, string>>("fetch_thumbnails", {
        fileIds: fileUuids,
      })
      return result
    } catch (error) {
      console.error("Error fetching thumbnails:", error)
      throw new Error("Failed to fetch thumbnails")
    }
  } catch (error) {
    console.error("Error fetching image UUIDs:", error)
    throw new Error("Failed to fetch image UUIDs")
  }
}
