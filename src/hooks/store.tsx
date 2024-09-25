import { create } from "zustand"

type RoverStore = {
  detailsViewFileUuid: string
  setDetailsViewFileUuid: (uuid: string) => void
  clearDetailsViewFileUuid: () => void

  pathPrefixes: string[]
  setPathPrefixes: (prefixes: string[]) => void

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

  pathPrefixes: [],
  setPathPrefixes: (prefixes) => {
    set(() => ({ pathPrefixes: prefixes }))
  },

  taskStatuses: new Map(),
  setTaskStatus: (uuid, status) => {
    set((state) => {
      // Note that we make a copy of the map here.
      // This is because updates to Zustand states must be
      // *immutable*. If we were to set() the original map,
      // we would not trigger a re-render.
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
