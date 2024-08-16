"use client"

import useRoverStore from "@/hooks/store"
import type FileUuid from "@/interfaces/FileUuid"
import type Thumbnail from "@/interfaces/Thumbnail"
import { useEffect, useState } from "react"
import { Masonry } from "react-plock"
import { fetchThumbnails, hnswSearch } from "../api"
import GalleryCard from "./GalleryCard"

interface GalleryProps {
  searchText: string
}

export const Gallery: React.FC<GalleryProps> = ({
  searchText,
}: GalleryProps) => {
  // Reasonable defaults for the number of neighbors and efArg for hnsw search
  // We can adjust these as needed for the user experience, including
  // cranking up the number of neighbors. Lag seems to be resulting from thumbnail loading
  // on the frontend, our HNSW search is very fast. We're addressing the lag in ROVER-116.
  const numberNeighbors = 500
  const efArg = 800
  // This is a somewhat arbitrary, large value. It will include results that are pretty semantically dissimilar.
  // But, we prefer to include errant results than exclude relevant ones. The results are ordered,
  // and the filtering on this threshold is done on the results of an (already very fast) HNSW search,
  // so this (1) doesn't add much time and (2) ensures we don't miss relevant results, with good results at the top anyways.
  // For context, ~0.75 is nearly a perfect search result, and ~0.85 is pretty semantically dissimilar.
  // The actual range of cosine distance values is -1 to 1, but this is a highly dimensional space, so distances tend
  // to be compressed into this range.
  const distanceThreshold = 0.85

  const [searchResults, setSearchResults] = useState<FileUuid[] | null>(null)

  useEffect(() => {
    const fetchSearchResults = async () => {
      try {
        const result = await hnswSearch(
          searchText,
          numberNeighbors,
          efArg,
          distanceThreshold,
        )
        setSearchResults(result)
      } catch (error) {
        console.error("Error fetching search results:", error)
      }
    }

    fetchSearchResults().catch((error: unknown) => {
      console.error(error)
    })
  }, [searchText])

  if (searchText !== "" && !searchResults) {
    // Indicate that the empty results are due to actually not having any results, rather than a loading state.
    // TODO - Once we have a marketplace, we can point them to it here.
    return <div className="text-center">No results found</div>
  }

  return searchResults ? <GalleryContent fileUuids={searchResults} /> : null
}

const GalleryContent: React.FC<{ fileUuids: FileUuid[] }> = ({ fileUuids }) => {
  const [thumbnails, setThumbnails] = useState<Thumbnail[] | null>(null)

  useEffect(() => {
    const getThumbnails = async () => {
      try {
        const result = await fetchThumbnails(fileUuids)
        setThumbnails(result)
      } catch (error) {
        console.error("Error fetching thumbnails:", error)
      }
    }

    getThumbnails().catch((error: unknown) => {
      console.error(error)
    })
  }, [fileUuids])

  const setDetailsViewFileUuid = useRoverStore(
    (state) => state.setDetailsViewFileUuid,
  )

  if (!thumbnails || thumbnails.length === 0) {
    return <div />
  }

  return (
    <Masonry
      items={thumbnails}
      config={{
        columns: [1, 2, 3, 4],
        gap: [24, 12, 6, 6],
        media: [640, 768, 1024, 2048],
      }}
      render={(item) => (
        <GalleryCard
          imageSrc={item.path}
          onClick={() => {
            setDetailsViewFileUuid(item.file_uuid)
          }}
        />
      )}
    />
  )
}
