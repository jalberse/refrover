"use client"

import type Thumbnail from "@/interfaces/Thumbnail"
import { Suspense, useEffect, useState } from "react"
import { fetchThumbnails } from "../api"
import { fetchMetadata } from "../api"
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
  const [thumbnails, setThumbnails] = useState<Thumbnail[] | null>(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const result = await fetchThumbnails(search_text)
        setThumbnails(result)
      } catch (error) {
        console.error(error)
      }
    }

    fetchData().catch((error: unknown) => {
      console.error(error)
    })
  }, [search_text])

  if (!thumbnails || thumbnails.length === 0) {
    return null
  }

  const columns: Thumbnail[][] = [[], [], [], []]
  thumbnails.forEach((thumbnail, index) => {
    columns[index % 4].push(thumbnail)
  })

  // Use the first element of each thumbnail as the key for the column
  const columnKeys = columns.map((column) => column[0].uuid)

  // TODO Next step is, instead of just logging metadata, display in a new component.
  //      AssetDetails or something.
  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
      {columns.map((column, columnIndex) => (
        <div key={columnKeys[columnIndex]} className="grid gap-4">
          {column.map((thumbnail) => (
            <GalleryCard
              key={thumbnail.uuid}
              imageSrc={thumbnail.path}
              onClick={() => {
                console.log(fetchMetadata(thumbnail.file_uuid))
              }}
            />
          ))}
        </div>
      ))}
    </div>
  )
}
