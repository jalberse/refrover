import AssetDetails from "@/components/AssetDetails"
import { Gallery } from "@/components/Gallery"
import Search from "@/components/Search"
import useRoverStore from "@/hooks/store"
import { useGlobalShortcut } from "@/hooks/tauri/shortcuts"
import Head from "next/head"
import Image from "next/image"
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

  return (
    <div className="flex flex-col bg-white">
      <Head>
        <title>RefRover: Build Your Visual Library</title>
        <meta name="RefRover" content="Reference Rover" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <div className="flex flex-1 flex-col items-center justify-center">
        <div className="flex max-w-3xl mx-auto p-4">
          <Search placeholder="Search for reference..." />
        </div>
      </div>

      <main className="flex flex-1 flex-col items-center justify-center py-8">
        <PanelGroup autoSaveId="persistence conditional" direction="horizontal">
          <Panel>
            <div style={{ overflow: "auto", padding: "8px" }}>
              <Gallery search_text={query} />
            </div>
          </Panel>
          <PanelResizeHandle />
          {isDetailsViewOpen && (
            <>
              <Panel className="border-l-2 border-light-grey-900">
                <div style={{ overflow: "auto", padding: "8px" }}>
                  <AssetDetails />
                </div>
              </Panel>
            </>
          )}
        </PanelGroup>
      </main>

      <footer className="flex flex-1 flex-grow-0 items-center justify-center border-t border-gray-200 py-4">
        <div>
          <a
            href="https://tauri.app/"
            target="_blank"
            rel="noopener noreferrer"
            className="flex flex-grow items-center justify-center p-4"
          >
            Powered by{" "}
            <span className="ml-2 h-6">
              <Image
                src="/tauri_logo_light.svg"
                alt="Vercel Logo"
                height={24}
                width={78}
              />
            </span>
          </a>
        </div>
      </footer>
    </div>
  )
}

export default Home
