import { create } from "zustand"

type RoverStore = {
  detailsViewFileUuid: string
  setDetailsViewFileUuid: (uuid: string) => void
  clearDetailsViewFileUuid: () => void

  fsEventStatus: string
  setFsEventStatus: (status: string) => void
  clearFsEventStatus: () => void
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
}))

export default useRoverStore
