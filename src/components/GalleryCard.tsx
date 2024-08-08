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
    <div onClick={onClick} onKeyUp={handleKeyUp}>
      <div>
        <img
          src={imageSrc}
          alt="Gallery Thumbnail"
          className="h-auto max-w-full rounded-lg shadow-md"
        />
      </div>
    </div>
  )
}

export default GalleryCard
