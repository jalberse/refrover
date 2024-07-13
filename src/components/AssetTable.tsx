interface AssetTableProps {
  search_text: string
  current_page: number
}

export const AssetTable: React.FC<AssetTableProps> = ({
  search_text,
  current_page,
}: AssetTableProps) => {
  return (
    <div className="flex flex-1 flex-col items-center justify-center py-8">
      <h1 className="m-0 text-center text-6xl">{search_text}</h1>
      <h1 className="m-0 text-center text-6xl">{current_page}</h1>
    </div>
  )
}
