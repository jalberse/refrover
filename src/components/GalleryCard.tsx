import type React from "react"

interface GalleryCardProps {
  imageSrc: string
  onClick: () => void
}

const GalleryCard: React.FC<GalleryCardProps> = ({ imageSrc, onClick }) => {
  const handleKeyUp = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" || event.key === " ") {
      onClick()
    }
  }

  return (
    <div
      className="rounded-lg overflow-hidden shadow-lg cursor-pointer"
      onClick={onClick}
      onKeyUp={handleKeyUp}
    >
      <img src={imageSrc} alt="Gallery Thumbnail" className="w-full h-auto" />
    </div>
  )
}

export default GalleryCard
