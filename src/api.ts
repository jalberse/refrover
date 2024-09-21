import { invoke } from "@tauri-apps/api/tauri"

import { convertFileSrc } from "@tauri-apps/api/tauri"
import type FileMetadata from "./interfaces/FileMetadata"
import type FileUuid from "./interfaces/FileUuid"
import type Thumbnail from "./interfaces/thumbnail"

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
export async function hnswSearch(
  queryString: string,
  numberNeighbors: number,
  efArg: number,
  distanceThreshold: number,
) {
  try {
    const fileUuids = await invoke<FileUuid[]>("search_images", {
      queryString,
      numberNeighbors,
      efArg,
      distanceThreshold,
    })
    return fileUuids
  } catch (error) {
    console.error("Error fetching image UUIDs:", error)
    throw new Error("Failed to fetch image UUIDs")
  }
}

// Fetches the thumbnails for the set of files with the resulting file UUIDs.
// If no thumbnail is available, it will be generated.
// This may take some time to execute as thumbnails are generated.
// We assume the caller knows the directory storing the thumbnails.
export async function fetchThumbnails(fileIds: FileUuid[]) {
  try {
    const thumbnails = await invoke<Thumbnail[]>("fetch_thumbnails", {
      fileIds: fileIds,
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
}

export async function addWatchedDirectory(directory: string) {
  try {
    await invoke("add_watched_directory", {
      directory,
    })
      .catch((error: unknown) => {
        console.error("Error adding watched directory:", error)
      })
      .then(() => {
        console.log("Successfully added watched directory")
      })
  } catch (error) {
    console.error("Error adding watched directory:", error)
  }
}

export async function deleteWatchedDirectory(directory: string) {
  try {
    await invoke("delete_watched_directory", {
      directory,
    })
      .catch((error: unknown) => {
        console.error("Error removing watched directory:", error)
      })
      .then(() => {
        console.log("Successfully removed watched directory")
      })
  } catch (error) {
    console.error("Error removing watched directory:", error)
  }
}

export function getWatchedDirectories() {
  return invoke<string[]>("get_watched_directories")
}
