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

  // TODO We may want to instead only open the details pane on double-click.
  //      Single click could just select the image, and shift+click adds to selection.
  //      This is important for dragging/copying images to reference programs.

  return (
    <div
      onClick={onClick}
      onKeyUp={handleKeyUp}
      style={{ display: "flex", alignItems: "center" }}
    >
      <img
        src={imageSrc}
        loading="lazy"
        alt="Gallery Thumbnail"
        className="h-auto max-w-full rounded-lg shadow-md"
        style={{ width: "100%" }}
      />
    </div>
  )
}

export default GalleryCard
