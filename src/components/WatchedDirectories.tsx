'use client'

import { open } from "@tauri-apps/api/dialog"
import React from "react"
import { useEffect, useRef, useState } from "react"
import { addWatchedDirectory, deleteWatchedDirectory, getWatchedDirectories } from "../api"
import { Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Button } from '@mui/material';

import { readDir, BaseDirectory } from '@tauri-apps/api/fs';

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
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMessage, setDialogMessage] = useState("");
  // Used in the dialog to show which directories were excluded
  const [excludedDirectories, setExcludedDirectories] = useState<string[]>([]);

  const [directoryTrees, setDirectoryTrees] = useState<DirectoryTreeItem[]>([]);
  const prevDirectoryTreesRef = useRef<DirectoryTreeItem[]>([]);

  const [selectedDirectories, setSelectedDirectories] = useState<string[]>([]);

  // On mount, get the initial set of watched directories from the database
  // Populate directoryTrees with the initial set of watched directories
  useEffect(() => {
    getWatchedDirectories().then((directories) => {
      getDirectoryTrees(directories).then((trees) => {
        setDirectoryTrees(trees);
        prevDirectoryTreesRef.current = trees
      }).catch((error: unknown) => {
        console.error("Error building directory trees:", error);
      })
    }).catch((error: unknown) => {
      console.error("Error fetching watched directories:", error);
    })
  }, [])

  const handleCloseDialog = () => {
    setDialogOpen(false);
  };

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

      const originalSelectedPath = selectedPath;

      // If any of the paths are already in the list, don't add them again
      selectedPath = selectedPath.filter((path) => !directoryTrees.some((dir) => dir.id === path))

      // If any of the paths are already *subdirectories* of a directory in the list, don't add them
      selectedPath = selectedPath.filter((path) => !directoryTrees.some((dir) => path.startsWith(dir.id)))

      // Similarly, if any of the paths to add are parents of directories in the list, don't add them
      selectedPath = selectedPath.filter((path) => !directoryTrees.some((dir) => dir.id.startsWith(path)))

      // If we've filtered out any paths, we want to let the user know via a modal dialog
      if (selectedPath.length < numberOfSelectedDirectories) {
        const excludedDirectories = originalSelectedPath.filter((path) => selectedPath && !selectedPath.includes(path));
        setDialogMessage(`Directories cannot be added if they are a subdirectory or parent of an existing watched directory. The following directories were not added:`);
        setExcludedDirectories(excludedDirectories);
        setDialogOpen(true);
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

  // const removeDirectory = (path: string) => {
  //   const newDirectories = directoryTrees.filter(
  //     (directory) => directory.id !== path,
  //   )
  //   setDirectoryTrees(newDirectories)
  // }

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

  // hmm, https://www.npmjs.com/package/react-arborist
  // react-arborist seems nice and definitely has onContextMenu. Maybe just go with that.
  //    Note that *what* context menu is still up in the air. Could do the tauri plugin, or use MUI's Menu.

  // TODO In the right click context menu, we want to have an option to open the directory in the file explorer.
  // For opening the file explorer, we probably want to use the shell:
  //   https://tauri.app/v1/api/js/shell/

  // TODO This may need to change according to react-arborist's API.
  const onSelectedItemsChange = (event: React.SyntheticEvent, itemIds: string[]) => {
    // TODO I think we may want to use a zustand store instead of this local state here, but I'm testing this out for now.
    setSelectedDirectories(itemIds);
    console.log(selectedDirectories);
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
      <Dialog
        open={dialogOpen}
        onClose={handleCloseDialog}
        aria-labelledby="alert-dialog-title"
        aria-describedby="alert-dialog-description"
      >
        <DialogTitle id="alert-dialog-title">{"Warning"}</DialogTitle>
        <DialogContent>
          <DialogContentText id="alert-dialog-description">
            {dialogMessage}
          </DialogContentText>
            <ul>
            {excludedDirectories.map((directory) => (
              <li key={directory}>
              <DialogContentText>
                {directory}
              </DialogContentText>
              </li>
            ))}
            </ul>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleCloseDialog} color="primary" autoFocus>
            OK
          </Button>
        </DialogActions>
      </Dialog>
    </div>
  )
}

export default WatchedDirectories