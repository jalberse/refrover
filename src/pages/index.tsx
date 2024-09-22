import AssetDetails from "@/components/AssetDetails"
import { Gallery } from "@/components/Gallery"
import Search from "@/components/Search"
import StatusBar from "@/components/StatusBar"
import WatchedDirectories from "@/components/WatchedDirectories"
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

  // TODO Limit the extent to which these panel groups can be resized.
  //      In the future maybe we want a more robust panel system,
  //      and e.g. make the search results column count change based on its panel
  //      size rather than the viewport (I tried following the docs on that,
  //      didn't quite work). But for now, just enforce our recommended layout and
  //      allow reasonable resizing.

  // TODO We'll have a tags component/panel below the Watched Directories panel.


  return (
    <div className="flex flex-col bg-white">
      <Head>
        <title>RefRover: Build Your Visual Library</title>
        <meta name="RefRover" content="Reference Rover" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main
        className="items-center justify-center h-full"
        style={{ height: "100vh" }}
      >
        <div className="flex max-w-3xl mx-auto p-4 h-1/10">
          <Search placeholder="Search for reference..." />
        </div>
        <PanelGroup
          className="flex-1 h-5/6"
          autoSaveId="persistence conditional"
          direction="horizontal"
          style={{ height: "90%" }}
        >
          <Panel id="LeftPanel" order={0} defaultSize={25}>
            <div className="flex-1 overflow-auto px-4 h-full">
              <WatchedDirectories />
            </div>
          </Panel>
          <PanelResizeHandle />
          <Panel id="Gallery" order={1}>
            <div className="flex-1 overflow-auto px-4 h-full">
              <Gallery searchText={query} />
            </div>
          </Panel>
          <PanelResizeHandle />
          {isDetailsViewOpen && (
            <>
              <Panel
                id="Details"
                order={2}
                defaultSize={25}
                className="flex-1 border-l-2 border-light-grey-900"
              >
                <div className="overflow-auto px-4 h-full">
                  <AssetDetails />
                </div>
              </Panel>
            </>
          )}
        </PanelGroup>
        <StatusBar />
      </main>

      <footer />
    </div>
  )
}

export default Home
