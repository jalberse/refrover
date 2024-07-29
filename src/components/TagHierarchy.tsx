import Box from "@mui/material/Box"
import { RichTreeView } from "@mui/x-tree-view/RichTreeView"
import type { TreeViewBaseItem } from "@mui/x-tree-view/models"
import * as React from "react"

const MUI_X_PRODUCTS: TreeViewBaseItem[] = [
  {
    id: "grid",
    label: "Data Grid",
    children: [
      { id: "grid-community", label: "@mui/x-data-grid" },
      { id: "grid-pro", label: "@mui/x-data-grid-pro" },
      { id: "grid-premium", label: "@mui/x-data-grid-premium" },
    ],
  },
  {
    id: "pickers",
    label: "Date and Time Pickers",
    children: [
      { id: "pickers-community", label: "@mui/x-date-pickers" },
      { id: "pickers-pro", label: "@mui/x-date-pickers-pro" },
    ],
  },
  {
    id: "charts",
    label: "Charts",
    children: [{ id: "charts-community", label: "@mui/x-charts" }],
  },
  {
    id: "tree-view",
    label: "Tree View",
    children: [{ id: "tree-view-community", label: "@mui/x-tree-view" }],
  },
]

export default function TagHierarchy() {
  // TODO Populate the table from the database. Our function we just made.
  //    But we want that data structure to include these id and label things now though...

  // TODO then how to create, move, edit, copy tags etc? Creating, deleting edges in various ways... I think there's
  //    documentation for that sort of thing in this lib.
  //    And how to extract which are currently selected? Unselected? I want the same interactions as eg
  //    the hierarchy in Maya or Blender. Need to figure out how to do that for this.

  return (
    <Box sx={{ minHeight: 352, minWidth: 250 }}>
      <RichTreeView items={MUI_X_PRODUCTS} />
    </Box>
  )
}
