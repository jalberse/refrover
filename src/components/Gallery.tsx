"use client"

// import Image from "next/image"
import { convertFileSrc } from "@tauri-apps/api/tauri"
import { Suspense, useEffect, useState } from "react"
import { fetchThumbnails } from "../api"
import GalleryCard from "./GalleryCard"

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
  // Group thumbnails into columns
  // Group thumbnails into columns
  const columns: string[][][] = [[], [], [], []]
  thumbnailFilepathsConverted.forEach((thumbnail, index) => {
    columns[index % 4].push(thumbnail)
  })

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
      {columns.map((column) => (
        <div key={column[0][0]} className="grid gap-4">
          {column.map((thumbnail) => (
            <GalleryCard
              key={thumbnail[0]}
              imageSrc={thumbnail[1]}
              onClick={() => {
                console.log(`Clicked on ${thumbnail[0]}`)
              }}
            />
          ))}
        </div>
      ))}
    </div>
  )
}
