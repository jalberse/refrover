'use client'

import { open } from "@tauri-apps/api/dialog"
import type React from "react"
import { useEffect, useRef, useState } from "react"
import { addWatchedDirectory, deleteWatchedDirectory } from "../api"
import WatchedDirectoriesList from "./WatchedDirectoriesList"

import * as fs from 'fs';
import * as path from 'path';
import { RichTreeView } from "@mui/x-tree-view"


export type Directory = {
  id: number
  path: string
}


type DirectoryTreeItem = {
  // TODO Perhaps generate IDs instead; we do string comparisons, but I'm not about to optimize for that yet.
  // The full path of the directory. Can be used as a unique ID.
  path: string;
  // TODO - potentially delete, and just call path.basename(path) where necessary?
  // The label to display in the hierarchy
  label: string;
  children?: DirectoryTreeItem[];
};

function getDirectoryTrees(directories: string[]): DirectoryTreeItem[] {
  const tree: DirectoryTreeItem[] = [];

  directories.forEach((dir) => {
    const dirTree = buildTree(dir);
    if (dirTree) {
      tree.push(dirTree);
    }
  });

  return tree;
}

function buildTree(dirPath: string): DirectoryTreeItem | null {
  const stats = fs.statSync(dirPath);
  if (!stats.isDirectory()) {
    return null;
  }

  const dirName = path.basename(dirPath);
  const treeItem: DirectoryTreeItem = {
    path: dirPath,
    label: dirName,
    children: [],
  };

  const items = fs.readdirSync(dirPath);
  items.forEach((item) => {
    const itemPath = path.join(dirPath, item);
    const itemStats = fs.statSync(itemPath);

    if (itemStats.isDirectory()) {
      const childTree = buildTree(itemPath);
      if (childTree) {
        treeItem.children?.push(childTree);
      }
    }
  });

  return treeItem;
}

const WatchedDirectories: React.FC = () => {
  // TODO We need to fetch the initial set of watched directories from the backend
  //      That will use getDirectoryTrees(), and should be in a useEffect() on mount (ie with empty dependency list)
  // TODO Note we may add directories if we e.g. drag files from file explorer in (we'd add its parent)
  //      (actually don't do that, but I think we *do* want a default directory. I think we'd handle that in the backend, though
  //       and we'd get that on mount from the DB from the above TODO)

  const [directoryTrees, setDirectoryTrees] = useState<DirectoryTreeItem[]>([]);
  const prevDirectoryTreesRef = useRef<DirectoryTreeItem[]>([]);

  // TODO We'll have DirectoryTreeItem[] in the state.
  //      We'll then use that to render the trees in the WatchedDirectoriesList component, modifying it to be like TagHierarchy to display those.
  //      On mount, we should fetch the directories from the backend and then build the tree from that.
  //      We can also update via the fs dialog to add a new watched directory.
  //      Finally, we'll also send an event from the fs watcher on the backend so that if a directory is added or removed within a watched directory,
  //          we can update the tree accordingly. We can probably just rebuild the whole thing, at least at first (we can surely send hints in the event though,
  //          just the path is good since we can jump to that location and insert it, I guess)
  // Once we have that, we can add directories to search by constructing the prefix search query from the tree and filtering the search results accordingly (URL query params).
  //      Buttons to add/remove dirs from search etc.
  //      SQL LIKE queries for prefix search make this simple.
  //      This is all far easier than explicitly tracking subdirs in the database, I think.
  // https://vizlib-io.atlassian.net/browse/ROVER-84

  const addDirectory = async () => {
    let selectedPath = await open({
      directory: true,
      multiple: true,
      title: "Select a directory",
    })

    if (selectedPath && typeof selectedPath === "string") {
      // Only one selected path; make it an array and fall through
      selectedPath = [selectedPath]
    }
    if (selectedPath && Array.isArray(selectedPath)) {
      const newDirectories = selectedPath.map((path) => {
        // Call buildTree() to get the DirectoryTreeItem
        const tree = buildTree(path);
        return tree;
      })
      .filter((dir) => dir !== null)
      setDirectoryTrees([...directoryTrees, ...newDirectories])
    }
  }

  const removeDirectory = (path: string) => {
    const newDirectories = directoryTrees.filter(
      (directory) => directory.path !== path,
    )
    setDirectoryTrees(newDirectories)
  }

  useEffect(() => {
    const prevDirectories = prevDirectoryTreesRef.current

    const addedDirectories = directoryTrees.filter(
      (dir) => !prevDirectories.some((prevDir) => prevDir.path === dir.path),
    )

    const removedDirectories = prevDirectories.filter(
      (prevDir) => !directoryTrees.some((dir) => dir.path === prevDir.path),
    )

    const addPromises = addedDirectories.map((dir) =>
      addWatchedDirectory(dir.path),
    )

    const removePromises = removedDirectories.map((dir) =>
      deleteWatchedDirectory(dir.path),
    )

    Promise.all([...addPromises, ...removePromises])
       .then(() => {
        prevDirectoryTreesRef.current = directoryTrees
       })
       .catch((error: unknown) => {
         console.error("Error updating watched directories:", error)
       })
  }, [directoryTrees])

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
      <RichTreeView items={directoryTrees} />
      {
        // <WatchedDirectoriesList
        // directories={directories}
        // removeDirectory={removeDirectory}
        // />
      }
    </div>
  )
}

export default WatchedDirectories
