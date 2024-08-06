"use client"

import { appDataDir } from "@tauri-apps/api/path"
import Image from "next/image"
import { Suspense, useEffect, useState } from "react"
import { fetchThumbnails } from "../api"

interface GalleryProps {
  search_text: string
}

export const Gallery: React.FC<GalleryProps> = ({
  search_text,
}: GalleryProps) => {
  // TODO Use a skeleton instead of "Loading..."
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <GalleryContent search_text={search_text} />
    </Suspense>
  )
}

const GalleryContent: React.FC<{ search_text: string }> = ({ search_text }) => {
  const [thumbnailFilenames, setThumbnailFilenames] = useState<
    Record<string, string>[] | null
  >(null)
  const [appDataDirPath, setAppDataDirPath] = useState<string | null>(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const result = await fetchThumbnails(search_text)
        // Ensure result is an array
        setThumbnailFilenames(Array.isArray(result) ? result : [result])
      } catch (error) {
        console.error(error)
      }
    }

    fetchData().catch((error: unknown) => {
      console.error(error)
    })
  }, [search_text])

  useEffect(() => {
    const fetchAppDataDir = async () => {
      try {
        const path = await appDataDir()
        setAppDataDirPath(path)
      } catch (error) {
        console.error(error)
      }
    }

    fetchAppDataDir().catch((error: unknown) => {
      console.error(error)
    })
  }, [])

  if (!thumbnailFilenames) {
    return null
  }

  if (!appDataDirPath) {
    return null
  }

  // TODO Joining OS paths in the browser is not a good idea.
  //      We should just return the full paths from Rust, including the appdata dir, and preprended with file://
  //      That also means we don't need the compelexity of fetching the appDataDirPath here.

  // The thumbnailFilenames are relative to the appDataDirPath.
  // thumbnailFilenames contains (UUID, filename) pairs.
  // Display them in a grid.
  return (
    <div className="grid grid-cols-3 gap-4">
      {thumbnailFilenames.map((thumbnailFilename) => {
        const [uuid, filename] = Object.entries(thumbnailFilename)[0]
        // Join the appDataDirPath with filename using the path module
        // const imagePath = path.join(appDataDirPath, filename);
        return (
          <div key={uuid} className="border border-gray-200 rounded-md p-2">
            <Image src={String(filename)} alt={uuid} width={200} height={200} />
          </div>
        )
      })}
    </div>
  )
}
