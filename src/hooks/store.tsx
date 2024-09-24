import { create } from "zustand"

type RoverStore = {
  detailsViewFileUuid: string
  setDetailsViewFileUuid: (uuid: string) => void
  clearDetailsViewFileUuid: () => void

  // TODO I think we want to instead have a set of IDs corresponding to currently-active-background-tasks.
  //      We can have two events (perhaps analyzer-start and analyzer-end), and each will send a UUID (generated at the start of the task,
  //      just before sending analyzer-start).
  //      The set maintains these UUIDs, removing them on analyzer-end and adding them on analyzer-start.
  //      When the set is non-empty, we know there's work being done and can display "rover-analyzer (spinner)".
  //      When the set is empty, we know there's no background work being done and can display "rover-analyzer (checkmark)".
  //      ... Or further, we can maintain a map of UUIDs to statuses that e.g. give the current progress of the task (number of images processed out of total, e.g.).
  //      A user can then click on the rover-analyzer button in the taskbar to view the list of tasks, and the status per.
  //   But maybe just start with the set of UUIDs and the spinner/checkmark status.
  fsEventStatus: string
  setFsEventStatus: (status: string) => void
  clearFsEventStatus: () => void

  pathPrefixes: string[]
  setPathPrefixes: (prefixes: string[]) => void
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
}))

export default useRoverStore
