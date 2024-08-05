import { invoke } from "@tauri-apps/api/tauri"

export async function fetchImages(queryString: string) {
  // TODO This seems a bit slow? It takes ~a second. I guess it's fine, but it still seems slow to me.
  // A search on ~1000 points in HNSW should not take this long I think.
  //   We should be able to pull up images almost instantly. I expected it to be slow in debug, but not in release.

  try {
    const result = await invoke<[number, string][]>("search_images", {
      queryString,
    })
    return result
  } catch (error) {
    throw new Error("Failed to fetch images")
  }
}
