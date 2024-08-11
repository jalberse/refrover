import AssetDetails from "@/components/AssetDetails"
import { Gallery } from "@/components/Gallery"
import Search from "@/components/Search"
import { useGlobalShortcut } from "@/hooks/tauri/shortcuts"
import Head from "next/head"
import Image from "next/image"
import { useSearchParams } from "next/navigation"
import { useCallback } from "react"
import { Suspense } from "react"

export const Home: React.FC = () => {
  // TODO I think there is a nicer way to do this since a Page should by
  // optionally accept the search params, but I couldn't figure it out.
  // That would be more type-safe, but this is fine for now.

  const searchParams = useSearchParams()
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const query = searchParams.get("query") ?? ""

  // TODO Delete this lol.
  const shortcutHandler = useCallback(() => {
    console.log("Ctrl+P was pressed!")
  }, [])
  useGlobalShortcut("CommandOrControl+P", shortcutHandler)

  // TODO So this page will have two components: Search and AssetTable.
  //    Uh, do those.
  // TODO Then AssetTable will display AssetCards via pagination.
  //    The content of the cards will be determined by passing the query
  //    (which is passed to the asset table, along with the page number)
  //    to the AssetTable, which will then call the query to construct the table.

  // TODO Want the tag hierarchy stuff on a left panel. Do that.

  return (
    <div className="flex min-h-screen flex-col bg-white">
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
        <div className="flex flex-wrap items-center justify-center px-4">
          <Suspense key={query} fallback={<div>Loading...</div>}>
            <Gallery search_text={query} />
          </Suspense>
          <AssetDetails />
        </div>
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
