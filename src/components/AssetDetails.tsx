import { fetchMetadata } from "@/api"
import useRoverStore from "@/hooks/store"
import { useEffect, useState } from "react"
import type FileMetadata from "../interfaces/FileMetadata"

const AssetDetails: React.FC = () => {
  const [fileMetadata, setFileMetadata] = useState<FileMetadata | null>(null)

  const detailsViewFileUuid = useRoverStore(
    (state) => state.detailsViewFileUuid,
  )

  console.log("detailsViewFileUuid", detailsViewFileUuid)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const result = await fetchMetadata(detailsViewFileUuid)

        console.log("result", result)

        if (result) {
          setFileMetadata(result)
        }
      } catch (error) {
        console.error(error)
      }
    }

    fetchData().catch((error: unknown) => {
      console.error(error)
    })
  })

  if (!fileMetadata) {
    return null
  }

  // TODO Additionally, style this whole component. And we want it to "stick" on the main screen, not scroll with the gallery.
  //      We also need some visual separator that makes it clear you can resize it.

  return (
    <div>
      <div className="flex justify-center p-4">
        <img src={fileMetadata.thumbnail_filepath} alt="Thumbnail" />
      </div>
      <div className="flex justify-center">
        <table className="table-auto">
          <tbody>
            <tr>
              <th className="text-right pr-4">Filename:</th>
              <td>{fileMetadata.filename}</td>
            </tr>
            {fileMetadata.image_type && (
              <tr>
                <th className="text-right pr-4">Image Type:</th>
                <td>{fileMetadata.image_type}</td>
              </tr>
            )}
            {fileMetadata.size && (
              <tr>
                <th className="text-right pr-4">Size:</th>
                <td>
                  {fileMetadata.size.width} x {fileMetadata.size.height}
                </td>
              </tr>
            )}
            {fileMetadata.date_created && (
              <tr>
                <th className="text-right pr-4">Created:</th>
                <td>{fileMetadata.date_created}</td>
              </tr>
            )}
            {fileMetadata.date_modified && (
              <tr>
                <th className="text-right pr-4">Modified:</th>
                <td>{fileMetadata.date_modified}</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}

export default AssetDetails
