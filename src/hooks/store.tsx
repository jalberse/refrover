import { create } from "zustand"

interface RoverStore {
  detailsViewFileUuid: string
  setDetailsViewFileUuid: (uuid: string) => void
  clearDetailsViewFileUuid: () => void
}

const useRoverStore = create<RoverStore>((set) => ({
  detailsViewFileUuid: "",
  setDetailsViewFileUuid: (uuid) => {
    set(() => ({ detailsViewFileUuid: uuid }))
  },
  clearDetailsViewFileUuid: () => {
    set(() => ({ detailsViewFileUuid: "" }))
  },
}))

export default useRoverStore
