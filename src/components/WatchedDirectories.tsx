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
  // TODO Note we may add directories if we e.g. drag files from file explorer in (we'd add its parent)
  const [directories, setDirectories] = useState<Directory[]>([])
  const prevDirectoriesRef = useRef<Directory[]>([])

  // TODO I think we want to display the tree for each watched directory instead. Users should be able to click through the trees of each watched directory to see the files within.
  //      See comment at https://vizlib-io.atlassian.net/browse/ROVER-84
  //      I don't think we need to like, include those subdirs in our database or anything.
  //      We should be able to display the tree here (listening for new subdirs though?)
  //      and then for search, we can search by *prefix* of the dirs most likely?
  // https://stackoverflow.com/questions/15020690/sqlilte-searching-with-prefix-string-by-using-match
  //      Yeah, that basically seems possible/fine.
  // Also, reference the Kindle search (and probably some other search aparatuses) for how we'd want to convey this.
  //      I'm thinking something like a "search" icon next to each dir, and then when you click it,
  //      a little bubble under our natural language search bar appears with "Starting With: [dir path]"
  //      Those are all treated as ORs I suppose.
  //      So you click in this hierarchy to add/remove dirs to search, and below the search bar we show
  //      which are active. Empty search + no dirs, empty gallery. Empty search + dir, images in that dir.
  //      Search + active dir, we only search in those active dirs.
  //      The *ordering* will come from the natural language search (if applicable).
  //      Tags, hen we add them after the MVP, will function similarly.
  // TODO But consider if a new dir is added to the watched directory.
  //      In our notify handler, we should probably just send an event any time a directory changes
  //      (rename, move, add, delete, etc.) and then on that event, we re-build this tree?
  //      We could track the subdirs in the database and then construct the tree from that, too...
  //      Not sure.
  //      Also - if a directory is removed, does it still fire all the file removal events?
  //       (or if it's added, does it fire all the file add events?)
  //       If the file events still trigger that's ideal, since we can just work with their absolute paths
  //       and say "it's in this watched directory" and not worry about subdir structure on the backend.
  //       We'd only need to worry about subdir structure for prefix searching, which is a simpler problem.

  // TODO hmm. gross.
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
