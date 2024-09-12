import type React from "react"
import { useEffect, useState } from "react"
import type { Directory } from "./WatchedDirectories"

type WatchedDirectoriesListProps = {
  directories: Directory[]
  removeDirectory: (id: number) => void
}

const WatchedDirectoriesList: React.FC<WatchedDirectoriesListProps> = ({
  directories,
  removeDirectory,
}) => {
  const [selectedDirectoryId, setSelectedDirectoryId] = useState<number | null>(
    null,
  )

  // TODO If the user clicks off somewhere else, the selected row should be deselected.

  const handleRowClick = (id: number) => {
    setSelectedDirectoryId(id)
  }

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Delete" && selectedDirectoryId !== null) {
        removeDirectory(selectedDirectoryId)
        setSelectedDirectoryId(null)
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    return () => {
      window.removeEventListener("keydown", handleKeyDown)
    }
  }, [removeDirectory, selectedDirectoryId])

  return (
    <table style={{ width: "100%", borderCollapse: "collapse" }}>
      <tbody>
        {directories.map((directory, index) => (
          <tr
            key={directory.id}
            onClick={() => {
              handleRowClick(directory.id)
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                handleRowClick(directory.id)
              }
            }}
            tabIndex={0}
            style={{
              backgroundColor:
                selectedDirectoryId === directory.id
                  ? "darkslategray"
                  : index % 2 === 0
                    ? "slategray"
                    : "lightslategray",
              color: "white",
              cursor: "pointer",
            }}
          >
            <td style={{ padding: "1px", border: "1px solid #ddd" }}>
              {directory.path}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

export default WatchedDirectoriesList
