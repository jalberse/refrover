import { create } from "zustand"

type RoverStore = {
  detailsViewFileUuid: string
  setDetailsViewFileUuid: (uuid: string) => void
  clearDetailsViewFileUuid: () => void

  // TODO Delete the fsEventStatus stuff... Task bar will now use taskStatuses.
  fsEventStatus: string
  setFsEventStatus: (status: string) => void
  clearFsEventStatus: () => void

  pathPrefixes: string[]
  setPathPrefixes: (prefixes: string[]) => void

  // TODO I'm thinking:
  // taskStatuses empty? "rover-analyzer (checkmark)"
  // taskStatuses non-empty? "rover-analyzer (spinner)"
  // Clicking the rover-analyzer will then show each of the "status" strings in a little
  //  modal list above it. The status strings from the backend can include the
  //  description and the numebr of images processed out of total.
  //  For now, that can just be in the given status string - the backend can format.
  //  In the future we might want to have a more structured status object
  //  so we can display things differently, but don't bother for now.

  // Create a map from UUIDs to statuses.
  taskStatuses: Map<string, string>
  setTaskStatus: (uuid: string, status: string) => void
  removeTaskStatus: (uuid: string) => void
}

const useRoverStore = create<RoverStore>((set) => ({
  detailsViewFileUuid: "",
  setDetailsViewFileUuid: (uuid) => {
    set(() => ({ detailsViewFileUuid: uuid }))
  },
  clearDetailsViewFileUuid: () => {
    set(() => ({ detailsViewFileUuid: "" }))
  },

  fsEventStatus: "rover-analyzer",
  setFsEventStatus: (status) => {
    set(() => ({ fsEventStatus: status }))
  },
  clearFsEventStatus: () => {
    set(() => ({ fsEventStatus: "" }))
  },

  pathPrefixes: [],
  setPathPrefixes: (prefixes) => {
    set(() => ({ pathPrefixes: prefixes }))
  },

  taskStatuses: new Map(),
  setTaskStatus: (uuid, status) => {
    set((state) => {
      const newTaskStatuses = new Map(state.taskStatuses)
      newTaskStatuses.set(uuid, status)
      return { taskStatuses: newTaskStatuses }
    })
  },
  removeTaskStatus: (uuid) => {
    set((state) => {
      const newTaskStatuses = new Map(state.taskStatuses)
      newTaskStatuses.delete(uuid)
      return { taskStatuses: newTaskStatuses }
    })
  },
}))

export default useRoverStore
