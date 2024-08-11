import useRoverStore from "@/hooks/store"

const AssetDetails: React.FC = () => {
  const detailsViewFileUuid = useRoverStore(
    (state) => state.detailsViewFileUuid,
  )

  if (!detailsViewFileUuid) {
    return <div />
  }

  return <div>{detailsViewFileUuid}</div>
}

export default AssetDetails
