/* eslint-disable @typescript-eslint/consistent-type-definitions */
import type React from "react"
import { useRef, useState } from "react"
import WatchedDirectoriesList from "./WatchedDirectoriesList"

declare module "react" {
  interface InputHTMLAttributes<T> extends HTMLAttributes<T> {
    // extends React's HTMLAttributes
    directory?: string
    webkitdirectory?: string
  }
}

export type Directory = {
  id: number
  path: string
}

const WatchedDirectories: React.FC = () => {
  const [directories, setDirectories] = useState<Directory[]>([])
  const fileInputRef = useRef<HTMLInputElement>(null)

  const generateUniqueId = () => {
    return Math.floor(Math.random() * 1000000)
  }

  const addDirectory = (event: React.ChangeEvent<HTMLInputElement>) => {
    const files = event.target.files
    if (files) {
      const directoryPath = files[0].webkitRelativePath.split("/")[0] // Get the directory name
      const newDirectory: Directory = {
        id: generateUniqueId(),
        path: directoryPath,
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

  const handleAddButtonClick = () => {
    if (fileInputRef.current) {
      fileInputRef.current.click()
    }
  }

  return (
    <div className="watched-directories">
      <h2>Watched Directories</h2>
      <WatchedDirectoriesList
        directories={directories}
        removeDirectory={removeDirectory}
      />
      <button
        type="button"
        className="bg-blue-500 text-white rounded mt-4"
        onClick={handleAddButtonClick}
      >
        +
      </button>
      <input
        ref={fileInputRef}
        type="file"
        directory=""
        webkitdirectory=""
        className="hidden"
        onChange={addDirectory}
      />
    </div>
  )
}

export default WatchedDirectories
