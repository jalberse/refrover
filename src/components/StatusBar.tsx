"use client"

import useRoverStore from "@/hooks/store"
import CheckIcon from "@mui/icons-material/Check"
import CircularProgress from "@mui/material/CircularProgress"
import { listen } from "@tauri-apps/api/event"
import type React from "react"
import { useEffect } from "react"
import type { TaskEndPayload, TaskStatusPayload } from "../interfaces/Payload"

async function startTaskStatusListener() {
  await listen<TaskStatusPayload>("task-status", (event) => {
    const setTaskStatus = useRoverStore.getState().setTaskStatus
    setTaskStatus(event.payload.uuid, event.payload.status)
  })
}

async function startTaskEndListener() {
  await listen<TaskEndPayload>("task-end", (event) => {
    const clearTaskStatus = useRoverStore.getState().removeTaskStatus
    clearTaskStatus(event.payload.uuid)
  })
}

const StatusBar: React.FC = () => {
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

  return (
    <div className="fixed bottom-0 w-full bg-blue-600 text-white text-left px-1 z-50">
      <span>rust-analyzer</span>
      {taskStatuses.size === 0 ? <CheckIcon /> : <CircularProgress />}
    </div>
  )
}

export default StatusBar
