import { open } from "@tauri-apps/api/dialog"
import type React from "react"
import { useEffect, useRef, useState } from "react"
import { addWatchedDirectory, deleteWatchedDirectory } from "../api"
import WatchedDirectoriesList from "./WatchedDirectoriesList"

export type Directory = {
  id: number
  path: string
}

const WatchedDirectories: React.FC = () => {
  // TODO We need to fetch the initial set of watched directories from the backend
  const [directories, setDirectories] = useState<Directory[]>([])
  const prevDirectoriesRef = useRef<Directory[]>([])

  const generateUniqueId = () => {
    return Math.floor(Math.random() * 1000000)
  }

  const addDirectory = async () => {
    const selectedPath = await open({
      directory: true,
      multiple: true,
      title: "Select a directory",
    })

    if (selectedPath && typeof selectedPath === "string") {
      const newDirectory: Directory = {
        id: generateUniqueId(),
        path: selectedPath,
      }
      setDirectories([...directories, newDirectory])
    }
    if (selectedPath && Array.isArray(selectedPath)) {
      const newDirectories = selectedPath.map((path) => ({
        id: generateUniqueId(),
        path,
      }))
      setDirectories([...directories, ...newDirectories])
    }
  }

  const removeDirectory = (id: number) => {
    const newDirectories = directories.filter(
      (directory) => directory.id !== id,
    )
    setDirectories(newDirectories)
  }

  useEffect(() => {
    const prevDirectories = prevDirectoriesRef.current

    const addedDirectories = directories.filter(
      (dir) => !prevDirectories.some((prevDir) => prevDir.id === dir.id),
    )

    const removedDirectories = prevDirectories.filter(
      (prevDir) => !directories.some((dir) => dir.id === prevDir.id),
    )

    // TODO Do we need to format the paths at all?
    const addPromises = addedDirectories.map((dir) =>
      addWatchedDirectory(dir.path),
    )
    const removePromises = removedDirectories.map((dir) =>
      deleteWatchedDirectory(dir.path),
    )

    Promise.all([...addPromises, ...removePromises])
      .then(() => {
        prevDirectoriesRef.current = directories
      })
      .catch((error: unknown) => {
        console.error("Error updating watched directories:", error)
      })
  }, [directories])

  return (
    <div>
      <button
        type="button"
        onClick={() => {
          // IIFE
          void (async () => {
            await addDirectory()
          })()
        }}
      >
        Add Directory
      </button>
      <WatchedDirectoriesList
        directories={directories}
        removeDirectory={removeDirectory}
      />
    </div>
  )
}

export default WatchedDirectories
