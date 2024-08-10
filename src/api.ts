import { invoke } from "@tauri-apps/api/tauri"

import type FileMetadata from "./interfaces/FileMetadata"
import type Thumbnail from "./interfaces/thumbnail"

// TODO Ensure we're using proper interfaces for e.g. File UUIDs.
//      I'm in a bad habit of using strings for everything on the frontend.

export async function fetchMetadata(fileUuid: string) {
  try {
    // Invoke the Tauri API to fetch the metadata for the given file UUID
    // The return value is a FileMetadata object serialized as JSON.
    const result = await invoke<string>("fetch_metadata", {
      fileId: fileUuid,
    })
    // Parse the JSON string into a FileMetadata object
    const metadata: FileMetadata = JSON.parse(result) as FileMetadata
    return metadata
  } catch (error) {
    console.log("Error fetching metadata:", error)
  }
}

// Fetches the thumbnails for the set of files with the given UUIDs.
// If no thumbnail is available, it will be generated.
// This may take some time to execute as thumbnails are generated.
// Returns a map from UUID to thumbnail image file names.
// We assume the caller knows the directory storing the thumbnails.
export async function fetchThumbnails(queryString: string) {
  try {
    const fileUuids = await invoke<[string][]>("search_images", {
      queryString,
    })

    console.log("fileUuids", fileUuids)

    try {
      const thumbnailMap = await invoke<Record<string, string>>(
        "fetch_thumbnails",
        {
          fileIds: fileUuids,
        },
      )

      // Map the result to a list of Thumbnail objects
      const thumbnails: Thumbnail[] = Object.entries(thumbnailMap).map(
        ([, thumb]) => ({
          uuid: thumb[0],
          filepath: thumb[1],
        }),
      )

      console.log("thumbnails", thumbnails)

      return thumbnails
    } catch (error) {
      console.error("Error fetching thumbnails:", error)
      throw new Error("Failed to fetch thumbnails")
    }
  } catch (error) {
    console.error("Error fetching image UUIDs:", error)
    throw new Error("Failed to fetch image UUIDs")
  }
}
