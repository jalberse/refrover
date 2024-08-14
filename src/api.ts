import { invoke } from "@tauri-apps/api/tauri"

import { convertFileSrc } from "@tauri-apps/api/tauri"
import type FileMetadata from "./interfaces/FileMetadata"
import type FileUuid from "./interfaces/FileUuid"
import type Thumbnail from "./interfaces/Thumbnail"

// TODO Ensure we're using proper interfaces for e.g. File UUIDs.
//      I'm in a bad habit of using strings for everything on the frontend.

export async function fetchMetadata(fileUuid: string) {
  try {
    // Invoke the Tauri API to fetch the metadata for the given file UUID
    // The return value is a FileMetadata object serialized as JSON.
    const result = await invoke<FileMetadata>("fetch_metadata", {
      fileId: fileUuid,
    })

    // Convert the thumbnail file path to a file URL
    result.thumbnail_filepath = convertFileSrc(result.thumbnail_filepath)

    return result
  } catch (error) {
    console.log("Error fetching metadata:", error)
  }
}

// Performs a KNN search using the HNSW index to find the nearest neighbors for the given query string.
//
// Fetches the thumbnails for the set of files with the resulting file UUIDs.
// If no thumbnail is available, it will be generated.
// This may take some time to execute as thumbnails are generated and the search is performed.
// We assume the caller knows the directory storing the thumbnails.
export async function hnswSearch(
  queryString: string,
  numberNeighbors: number,
  efArg: number,
) {
  try {
    const fileUuids = await invoke<FileUuid[]>("search_images", {
      queryString,
      numberNeighbors,
      efArg,
    })

    try {
      const thumbnails = await invoke<Thumbnail[]>("fetch_thumbnails", {
        fileIds: fileUuids,
      })

      const thumbnailFilepathsConverted: Thumbnail[] = thumbnails.map(
        (thumbnail) => {
          return {
            uuid: thumbnail.uuid,
            file_uuid: thumbnail.file_uuid,
            path: convertFileSrc(thumbnail.path),
          }
        },
      )

      return thumbnailFilepathsConverted
    } catch (error) {
      console.error("Error fetching thumbnails:", error)
      throw new Error("Failed to fetch thumbnails")
    }
  } catch (error) {
    console.error("Error fetching image UUIDs:", error)
    throw new Error("Failed to fetch image UUIDs")
  }
}
