// TODO We'll want a status bar that runs across the bottom of the screen.
// At first, this can be used just for displaying the status of handling new photos etc.

import useRoverStore from "@/hooks/store"
import { listen } from "@tauri-apps/api/event"
import type React from "react"
import type Payload from "../interfaces/Payload"

async function startListener() {
  await listen<Payload>("fs-event", (event) => {
    // TODO Why am I getting this? We are able to set it fine from rust (yay!) but this is new.
    //ReferenceError: window is not defined
    // at uid (file:///D:/projects/RefRover/refrover/node_modules/.pnpm/@tauri-apps+api@1.5.6/node_modules/@tauri-apps/api/tauri.js:6:5)
    //   at transformCallback (file:///D:/projects/RefRover/refrover/node_modules/.pnpm/@tauri-apps+api@1.5.6/node_modules/@tauri-apps/api/tauri.js:17:24)
    //   at listen (file:///D:/projects/RefRover/refrover/node_modules/.pnpm/@tauri-apps+api@1.5.6/node_modules/@tauri-apps/api/helpers/event.js:58:22)
    //   at listen (file:///D:/projects/RefRover/refrover/node_modules/.pnpm/@tauri-apps+api@1.5.6/node_modules/@tauri-apps/api/event.js:59:12)
    //   at startListener (webpack-internal:///./src/components/StatusBar.tsx:19:72)
    //   at StatusBar (webpack-internal:///./src/components/StatusBar.tsx:28:5)
    useRoverStore.setState({ fsEventStatus: event.payload.message })
  })
}

const StatusBar: React.FC = () => {
  startListener().catch((error: unknown) => {
    console.error(error)
  })

  const fsEventStatus: string = useRoverStore((state) => state.fsEventStatus)

  return (
    <div className="fixed bottom-0 w-full bg-blue-600 text-white text-left px-1 z-50">
      <span>{fsEventStatus}</span>
    </div>
  )
}

export default StatusBar
