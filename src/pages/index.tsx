import AssetDetails from "@/components/AssetDetails"
import { Gallery } from "@/components/Gallery"
import Search from "@/components/Search"
import useRoverStore from "@/hooks/store"
import { useGlobalShortcut } from "@/hooks/tauri/shortcuts"
import Head from "next/head"
import { useSearchParams } from "next/navigation"
import { useCallback } from "react"
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels"

export const Home: React.FC = () => {
  const searchParams = useSearchParams()
  const query = searchParams.get("query") ?? ""

  const clearDetailsViewFileUuid = useRoverStore(
    (state) => state.clearDetailsViewFileUuid,
  )

  const shortcutHandler = useCallback(() => {
    clearDetailsViewFileUuid()
  }, [clearDetailsViewFileUuid])
  useGlobalShortcut("Esc", shortcutHandler)

  const detailsViewFileUuid = useRoverStore(
    (state) => state.detailsViewFileUuid,
  )
  const isDetailsViewOpen = detailsViewFileUuid !== ""

  // TODO We want to (1) avoid style, and stick to tailwind classes
  //      and (2) prevent the scrollbar from appearing at the top level.
  //      We just want to scroll vertically within the gallery, or the detail pain if it's squished.

  return (
    <div className="flex flex-col bg-white" style={{ height: "100vh" }}>
      <Head>
        <title>RefRover: Build Your Visual Library</title>
        <meta name="RefRover" content="Reference Rover" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main
        className="flex-1 items-center justify-center "
        style={{ height: "100vh" }}
      >
        <div className="items-center justify-center">
          <div className="flex max-w-3xl mx-auto p-4">
            <Search placeholder="Search for reference..." />
          </div>
        </div>

        <PanelGroup
          className="flex-1"
          autoSaveId="persistence conditional"
          direction="horizontal"
        >
          <Panel>
            <div className="flex-1 overflow-auto p-8 h-full">
              <Gallery search_text={query} />
            </div>
          </Panel>
          <PanelResizeHandle />
          {isDetailsViewOpen && (
            <>
              <Panel className="flex-1 border-l-2 border-light-grey-900">
                <div className="overflow-auto p-8 h-full">
                  <AssetDetails />
                </div>
              </Panel>
            </>
          )}
        </PanelGroup>
      </main>

      <footer />
    </div>
  )
}

export default Home
