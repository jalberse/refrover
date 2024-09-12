/* eslint-disable @typescript-eslint/consistent-type-definitions */
import type React from "react"
import { useRef, useState } from "react"

declare module "react" {
  interface InputHTMLAttributes<T> extends HTMLAttributes<T> {
    // extends React's HTMLAttributes
    directory?: string
    webkitdirectory?: string
  }
}

interface Directory {
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
    // Clear the input value to allow re-selection of the same directory
    event.target.value = ""
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
      <ul>
        {directories.map((directory) => (
          <li key={directory.id} className="flex justify-between items-center">
            <span>{directory.path}</span>
            <button
              type="button"
              className="bg-red-500 text-white px-2 py-1 rounded"
              onClick={() => {
                removeDirectory(directory.id)
              }}
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
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
