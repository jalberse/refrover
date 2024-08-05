"use client"

import { Suspense, useEffect, useState } from "react"
import { fetchImages } from "../api"

interface AssetTableProps {
  search_text: string
}

export const AssetTable: React.FC<AssetTableProps> = ({
  search_text,
}: AssetTableProps) => {
  // TODO Use a skeleton instead of "Loading..."
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <AssetTableContent search_text={search_text} />
    </Suspense>
  )
}

const AssetTableContent: React.FC<{ search_text: string }> = ({
  search_text,
}) => {
  const [data, setData] = useState<[number, string][] | null>(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const result = await fetchImages(search_text)
        setData(result)
      } catch (error) {
        console.error(error)
      }
    }

    fetchData().catch((error: unknown) => {
      console.error(error)
    })
  }, [search_text])

  if (!data) {
    return null
  }

  // TODO We will replace the simple img tag with some "GalleryCard" component instead.
  //   I'm sure there's plenty of examples.
  //   Also need a thumbnailing system: see VIZLIB-57.

  return (
    <div className="grid grid-cols-3 gap-4">
      {data.map((imageBase64) => (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          key={imageBase64[0]}
          src={imageBase64[1]}
          alt={String(imageBase64[0])}
        />
      ))}
    </div>
  )
}
