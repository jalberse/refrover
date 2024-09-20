'use client'

import { open } from "@tauri-apps/api/dialog"
import type React from "react"
import { useEffect, useRef, useState } from "react"
import { addWatchedDirectory, deleteWatchedDirectory } from "../api"
import WatchedDirectoriesList from "./WatchedDirectoriesList"

import { readDir, BaseDirectory } from '@tauri-apps/api/fs';
import { RichTreeView } from "@mui/x-tree-view"


export type Directory = {
  id: number
  path: string
}


type DirectoryTreeItem = {
  // TODO Perhaps generate IDs instead; we do string comparisons, but I'm not about to optimize for that yet.
  // The full path of the directory. Can be used as a unique ID.
  id: string;
  // The label to display in the hierarchy
  label: string;
  children?: DirectoryTreeItem[];
};

async function buildTree(dir: string): Promise<DirectoryTreeItem | null> {
  // Dynamically import due to restrictions with Next.js SSR + Tauri.
  // (We don't use SSR, but we still have to use dynamic imports to avoid the error.)
  const basename = (await import ('@tauri-apps/api/path')).basename;

  try {
    const entries = await readDir(dir, { dir: BaseDirectory.App });
    const children = await Promise.all(entries.map(async (entry) => {
      if (entry.children) {
        return await buildTree(entry.path);
      }
      return null;
    }));

    return {
      id: dir,
      label: await basename(dir),
      children: children.filter((child) => child !== null) as DirectoryTreeItem[],
    };
  } catch (error) {
    console.error(`Error reading directory ${dir}:`, error);
    return null;
  }
}

// TODO We'll use this for the initial population
async function getDirectoryTrees(directories: string[]): Promise<DirectoryTreeItem[]> {
  const tree: DirectoryTreeItem[] = [];

  for (const dir of directories) {
    const dirTree = await buildTree(dir);
    if (dirTree) {
      tree.push(dirTree);
    }
  }

  return tree;
}

const WatchedDirectories: React.FC = () => {
  // TODO We need to fetch the initial set of watched directories from the backend
  //      That will use getDirectoryTrees(), and should be in a useEffect() on mount (ie with empty dependency list)
  // TODO Note we may add directories if we e.g. drag files from file explorer in (we'd add its parent)
  //      (actually don't do that, but I think we *do* want a default directory. I think we'd handle that in the backend, though
  //       and we'd get that on mount from the DB from the above TODO)

  const [directoryTrees, setDirectoryTrees] = useState<DirectoryTreeItem[]>([]);
  const prevDirectoryTreesRef = useRef<DirectoryTreeItem[]>([]);

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

      const numberOfSelectedDirectories = selectedPath.length;

      // If any of the paths are already in the list, don't add them again
      selectedPath = selectedPath.filter((path) => !directoryTrees.some((dir) => dir.id === path))

      // If any of the paths are already *subdirectories* of a directory in the list, don't add them
      selectedPath = selectedPath.filter((path) => !directoryTrees.some((dir) => path.startsWith(dir.id)))

      // Similarly, if any of the paths to add are parents of directories in the list, don't add them
      selectedPath = selectedPath.filter((path) => !directoryTrees.some((dir) => dir.id.startsWith(path)))

      // If we've filtered out any paths, we want to let the user know via a pop-up
      if (selectedPath.length < numberOfSelectedDirectories) {
        // TODO Show a pop-up
        console.log("Some directories were already added or are subdirectories of directories already added.")
      }

      const newDirectories = await Promise.all(selectedPath.map((path) => {
        const tree = buildTree(path);
        return tree;
      })).catch((error: unknown) => {
        console.error("Error adding directory:", error);
        return [];
      }).then((trees) => trees.filter((tree) => tree !== null) as DirectoryTreeItem[]);
      setDirectoryTrees([...directoryTrees, ...newDirectories])
    }
  }

  const removeDirectory = (path: string) => {
    const newDirectories = directoryTrees.filter(
      (directory) => directory.id !== path,
    )
    setDirectoryTrees(newDirectories)
  }

  useEffect(() => {
    const prevDirectories = prevDirectoryTreesRef.current

    const addedDirectories = directoryTrees.filter(
      (dir) => !prevDirectories.some((prevDir) => prevDir.id === dir.id),
    )

    const removedDirectories = prevDirectories.filter(
      (prevDir) => !directoryTrees.some((dir) => dir.id === prevDir.id),
    )

    const addPromises = addedDirectories.map((dir) =>
      addWatchedDirectory(dir.id),
    )

    const removePromises = removedDirectories.map((dir) =>
      deleteWatchedDirectory(dir.id),
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
