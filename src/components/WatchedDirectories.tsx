/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-call */
'use client'

import { open } from "@tauri-apps/api/dialog"
import React, { forwardRef } from "react"
import { useEffect, useRef, useState } from "react"
import { addWatchedDirectory, deleteWatchedDirectory, getWatchedDirectories } from "../api"
import { Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Button, Menu, MenuItem } from '@mui/material';

import { readDir, BaseDirectory } from '@tauri-apps/api/fs';

import { styled } from '@mui/material/styles';
import Box from '@mui/material/Box';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import { UseTreeItem2Parameters } from '@mui/x-tree-view/useTreeItem2';
import {
  TreeItem2Content,
  TreeItem2IconContainer,
  TreeItem2GroupTransition,
  TreeItem2Label,
  TreeItem2Root,
  TreeItem2Checkbox,
} from '@mui/x-tree-view/TreeItem2';
import { TreeItem2Icon } from '@mui/x-tree-view/TreeItem2Icon';
import { TreeItem2Provider } from '@mui/x-tree-view/TreeItem2Provider';
import { useTreeItem2 } from "@mui/x-tree-view/useTreeItem2"
import useRoverStore from "@/hooks/store"

export type Directory = {
  id: number
  path: string
}

type DirectoryTreeItem = {
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
      children: children.filter((child) => child !== null),
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

  const [contextMenu, setContextMenu] = useState<{ mouseX: number; mouseY: number, itemId: string } | null>(null);

  const setPathPrefixes = useRoverStore((state) => state.setPathPrefixes)

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

  const removeDirectory = (itemId: string) => {
    const newDirectories = directoryTrees.filter(
      (directory) => directory.id !== itemId,
    )
    // Remove the directory from the list of selected directories if it was selected
    setSelectedDirectories(selectedDirectories.filter((directory) => directory !== itemId));
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

  // TODO In the right click context menu, we want to have an option to open the directory in the file explorer.
  // For opening the file explorer, we probably want to use the shell:
  //   https://tauri.app/v1/api/js/shell/

  // TODO ... but before that, some "copy path" option is probably easy to start with.

  const onSelectedItemsChange = (event: React.SyntheticEvent, itemIds: string[]) => {
    // TODO - We should be able to delete the selectedDirectories setting here,
    //       and that whole local state. The zustand store will be sufficient.
    setSelectedDirectories(itemIds);
    setPathPrefixes(itemIds);
  }

  const onContextMenuHandler = (event: React.MouseEvent<HTMLDivElement>, itemId: string) => {
    event.preventDefault();
    setContextMenu(
      contextMenu === null
        ? {
            mouseX: event.clientX - 2,
            mouseY: event.clientY - 4,
            itemId: itemId,
          }
        : null,
    );
  }

  const handleContextMenuClose = () => {
    setContextMenu(null);
  };

  const isRootEntry = (itemId: string) => {
    return directoryTrees.some((directory) => directory.id === itemId);
  };

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
      <RichTreeView
        items={directoryTrees}
        expansionTrigger="iconContainer"
        checkboxSelection={true}
        multiSelect
        onSelectedItemsChange={onSelectedItemsChange}
        slots={{ item: CustomTreeItem }}
        // Note that "as any" is unforunately required/recommended here by MUI, unless they've made a fix.
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        slotProps={{ item: { onContextMenuHandler } as any }}
      />
      <Menu
        open={contextMenu !== null}
        onClose={handleContextMenuClose}
        anchorReference="anchorPosition"
        anchorPosition={
          contextMenu !== null
            ? { top: contextMenu.mouseY, left: contextMenu.mouseX }
            : undefined
        }
      >
        {contextMenu && isRootEntry(contextMenu.itemId) && (
          <MenuItem
            onClick={() => {
              removeDirectory(contextMenu.itemId);
              handleContextMenuClose();
            }}
          >
            Delete
          </MenuItem>
        )}
      </Menu>
      {
        // Just print the selected directories for now
        selectedDirectories.map((directory) => (
          <div key={directory}>
            {directory}
          </div>
        ))
      }
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

const CustomTreeItemContent = styled(TreeItem2Content)(({ theme }) => ({
  padding: theme.spacing(0.5, 1),
}));

// eslint-disable-next-line @typescript-eslint/consistent-type-definitions
interface CustomTreeItemProps
  extends Omit<UseTreeItem2Parameters, 'rootRef'>,
    Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> {
      onContextMenuHandler?: (event: React.MouseEvent<HTMLDivElement>, itemId: string) => void;
    }

// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, react/display-name
const CustomTreeItem = forwardRef((
  props: CustomTreeItemProps,
  ref: React.Ref<HTMLLIElement>,
) => {
  const { id, itemId, label, disabled, children, onContextMenuHandler, ...other } = props;

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const {
    getRootProps,
    getContentProps,
    getIconContainerProps,
    getCheckboxProps,
    getLabelProps,
    getGroupTransitionProps,
    status,
  // eslint-disable-next-line @typescript-eslint/no-unsafe-call
  } = useTreeItem2({ id, itemId, children, label, disabled, rootRef: ref });

  return (
    <TreeItem2Provider itemId={itemId}>
      <TreeItem2Root {...getRootProps(other)}>
        <CustomTreeItemContent {...getContentProps()}>
          <TreeItem2IconContainer {...getIconContainerProps()}>
            <TreeItem2Icon status={status} />
          </TreeItem2IconContainer>
          <Box
            sx={{ flexGrow: 1, display: 'flex', gap: 1 }}
            onContextMenu={(event: React.MouseEvent<HTMLDivElement>) => onContextMenuHandler?.(event, itemId)}
          >
            <TreeItem2Checkbox {...getCheckboxProps()} />
            <TreeItem2Label {...getLabelProps()} />
          </Box>
        </CustomTreeItemContent>
        {children && <TreeItem2GroupTransition {...getGroupTransitionProps()} />}
      </TreeItem2Root>
    </TreeItem2Provider>
  )
})

export default WatchedDirectories
