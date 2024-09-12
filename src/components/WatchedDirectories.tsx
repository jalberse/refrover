import { open } from "@tauri-apps/api/dialog"
import type React from "react"
import { useState } from "react"
import WatchedDirectoriesList from "./WatchedDirectoriesList"

export type Directory = {
  id: number
  path: string
}

const WatchedDirectories: React.FC = () => {
  const [directories, setDirectories] = useState<Directory[]>([])

  const generateUniqueId = () => {
    return Math.floor(Math.random() * 1000000)
  }

  // TODO We want to allow multiple directories to be added at once.
  const addDirectory = async () => {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: "Select a directory",
    })

    if (selectedPath && typeof selectedPath === "string") {
      const newDirectory: Directory = {
        id: generateUniqueId(),
        path: selectedPath,
      }
      setDirectories([...directories, newDirectory])
    }
  }

  const removeDirectory = (id: number) => {
    const newDirectories = directories.filter(
      (directory) => directory.id !== id,
    )
    setDirectories(newDirectories)
  }

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
