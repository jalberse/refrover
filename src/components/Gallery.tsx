"use client"

// import Image from "next/image"
import { convertFileSrc } from "@tauri-apps/api/tauri"
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
  const [thumbnailFilepaths, setThumbnailFilepaths] = useState<
    Record<string, string>[] | null
  >(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const result = await fetchThumbnails(search_text)
        console.log(result)
        // Ensure result is an array
        setThumbnailFilepaths(Array.isArray(result) ? result : [result])
      } catch (error) {
        console.error(error)
      }
    }

    fetchData().catch((error: unknown) => {
      console.error(error)
    })
  }, [search_text])

  if (!thumbnailFilepaths) {
    return null
  }

  // TODO - Actually, move this into the api. We should do any necessary conversion there. I'm just working on the parallel thumbnail creation for now.
  const thumbnailFilepathsConverted = thumbnailFilepaths.map((thumbnail) => {
    return [thumbnail[0], convertFileSrc(thumbnail[1])]
  })

  // fetchThumbnails returns a an array of arrays, where each subarray is a (UUID, thumbnail path) pair.
  // Display them in a grid, using the UUID as the ID for the image.
  return (
    <div className="grid grid-cols-3 gap-4">
      {thumbnailFilepathsConverted.map((thumbnail) => (
        <img
          key={thumbnail[0]}
          src={thumbnail[1]}
          alt={thumbnail[0]}
          className="gallery-image"
        />
      ))}
    </div>
  )
}
