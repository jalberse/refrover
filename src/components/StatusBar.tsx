"use client"

import useRoverStore from "@/hooks/store"
import { listen } from "@tauri-apps/api/event"
import type React from "react"
import { useEffect } from "react"
import type Payload from "../interfaces/Payload"

async function startListener() {
  await listen<Payload>("fs-event", (event) => {
    // TODO Rather than just setting text from the backend, I'd rather have an enum of statuses
    // and have the frontend choose how they are rendered.
    // I want a "rover-analyzer (checkmark)" status, and a "rover-analyzer (spinner)" status when
    // it's doing work.
    useRoverStore.setState({ fsEventStatus: event.payload.message })
  })
}

const StatusBar: React.FC = () => {
  useEffect(() => {
    startListener().catch((error: unknown) => {
      console.error(error)
    })
  }, [])

  const fsEventStatus: string = useRoverStore((state) => state.fsEventStatus)

  return (
    <div className="fixed bottom-0 w-full bg-blue-600 text-white text-left px-1 z-50">
      <span>{fsEventStatus}</span>
    </div>
  )
}

export default StatusBar
