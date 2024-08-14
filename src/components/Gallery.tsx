"use client"

import useRoverStore from "@/hooks/store"
import type Thumbnail from "@/interfaces/Thumbnail"
import { useEffect, useState } from "react"
import { hnswSearch } from "../api"
import GalleryCard from "./GalleryCard"

interface GalleryProps {
  search_text: string
}

export const Gallery: React.FC<GalleryProps> = ({
  search_text,
}: GalleryProps) => {
  return <GalleryContent search_text={search_text} />
}

const GalleryContent: React.FC<{ search_text: string }> = ({ search_text }) => {
  const [thumbnails, setThumbnails] = useState<Thumbnail[] | null>(null)
  const setDetailsViewFileUuid = useRoverStore(
    (state) => state.setDetailsViewFileUuid,
  )

  // Reasonable defaults for the number of neighbors and efArg.
  // We can adjust these as needed for the user experience, including
  // cranking up the number of neighbors. Lag seems to be resulting from thumbnail loading
  // on the frontend, our HNSW search is very fast. We're addressing the lag in ROVER-116.
  const numberNeighbors = 500
  const efArg = 800

  useEffect(() => {
    const fetchData = async () => {
      try {
        const result = await hnswSearch(search_text, numberNeighbors, efArg)
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

  // TODO Additionally, perhaps pressing escape should clear that state so that we remove the details component.
  //      I had a ctrl + P example for doing shortcuts/button inputs like that somewhere.

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
      {columns.map((column, columnIndex) => (
        <div key={columnKeys[columnIndex]} className="grid gap-4">
          {column.map((thumbnail) => (
            <GalleryCard
              key={thumbnail.uuid}
              imageSrc={thumbnail.path}
              onClick={() => {
                setDetailsViewFileUuid(thumbnail.file_uuid)
              }}
            />
          ))}
        </div>
      ))}
    </div>
  )
}
