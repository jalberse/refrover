interface AssetTableProps {
  search_text: string
}

export const AssetTable: React.FC<AssetTableProps> = ({
  search_text,
}: AssetTableProps) => {
  return (
    <div className="flex flex-1 flex-col items-center justify-center py-8">
      <h1 className="m-0 text-center text-6xl">{search_text}</h1>
    </div>
  )
}
