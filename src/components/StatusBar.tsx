"use client"

import useRoverStore from "@/hooks/store"
import CheckIcon from "@mui/icons-material/Check"
import { Button, Popover, Typography } from "@mui/material"
import CircularProgress from "@mui/material/CircularProgress"
import { listen } from "@tauri-apps/api/event"
import React from "react"
import { useEffect } from "react"
import type { TaskEndPayload, TaskStatusPayload } from "../interfaces/Payload"

async function startTaskStatusListener() {
  await listen<TaskStatusPayload>("task-status", (event) => {
    console.log("task status: ", event)
    const setTaskStatus = useRoverStore.getState().setTaskStatus
    setTaskStatus(event.payload.task_uuid, event.payload.status)
    const taskStatusMap = useRoverStore.getState().taskStatuses
    console.log("task status map: ", taskStatusMap)
  })
}

async function startTaskEndListener() {
  await listen<TaskEndPayload>("task-end", (event) => {
    console.log("task end: ", event)
    const clearTaskStatus = useRoverStore.getState().removeTaskStatus
    clearTaskStatus(event.payload.task_uuid)
  })
}

const StatusBar: React.FC = () => {
  const [anchorEl, setAnchorEl] = React.useState<HTMLButtonElement | null>(null)

  useEffect(() => {
    startTaskStatusListener().catch((error: unknown) => {
      console.error(error)
    })
  }, [])

  useEffect(() => {
    startTaskEndListener().catch((error: unknown) => {
      console.error(error)
    })
  }, [])

  const taskStatuses = useRoverStore((state) => state.taskStatuses)

  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    taskStatuses.size > 0 && setAnchorEl(event.currentTarget)
  }

  const handleClose = () => {
    setAnchorEl(null)
  }

  const open = Boolean(anchorEl)
  const id = open ? "rust-analyzer-popover" : undefined

  // TODO This is working ~okay~ but if an error occurs (in this case because we erroneously
  // call add_watched_directory twice, so it already exists the second time, which we should fix),
  // then the store never clears the task status.
  // One solution would be to pass the task uuid from the frontend into the command,
  // so that if an error occurs we can clear the task status from the store with that UUID.
  //   (or return the UUID in the error message, and then clear the store based on that)
  // The latter seems a bit more idiomatic.
  // Something like this:
  // https://github.com/tauri-apps/tauri/discussions/6952
  // And then we'd use that error in api.tsx to clear the provided taskStatus from the store.

  // TODO This would be on the backend, but the messages are too long. I suspect just the name (not absolute path) of the new dir is sufficient.
  //      (we could also have a structured response so the frontend could decide, but I'm lazy)

  return (
    <div className="fixed bottom-0 w-full bg-blue-600 text-white text-left px-1 z-50">
      <Button
        aria-describedby={id}
        variant="contained"
        color="primary"
        onClick={handleClick}
      >
        <span>rust-analyzer</span>
        <span style={{ padding: "0 4px" }}>
          {taskStatuses.size === 0 ? (
            <CheckIcon />
          ) : (
            <CircularProgress size="1rem" />
          )}
        </span>
      </Button>
      <Popover
        id={id}
        open={open}
        anchorEl={anchorEl}
        onClose={handleClose}
        anchorOrigin={{
          vertical: "top",
          horizontal: "left",
        }}
        transformOrigin={{
          vertical: "bottom",
          horizontal: "left",
        }}
      >
        <Typography sx={{ p: 2 }}>
          {Array.from(taskStatuses.entries()).map(([taskId, status]) => (
            <div key={taskId}>{status}</div>
          ))}
        </Typography>
      </Popover>
    </div>
  )
}

export default StatusBar
