"use client"

import { usePathname, useRouter, useSearchParams } from "next/navigation"
import { useDebouncedCallback } from "use-debounce"

export default function Search({ placeholder }: { placeholder: string }) {
  const searchParams = useSearchParams()
  const pathname = usePathname()
  // eslint-disable-next-line @typescript-eslint/unbound-method
  const { replace } = useRouter()

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call
  const handleSearch = useDebouncedCallback((term) => {
    // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
    const params = new URLSearchParams(searchParams)
    if (term) {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      params.set("query", term)
    } else {
      params.delete("query")
    }
    replace(`${pathname}?${params.toString()}`)
  }, 300)

  // TODO I think we also want to display badges for the prefixed paths (and later tags), which we can get from the store.
  //      Those badges can have a "X" button to remove them (which would also need to effect their hierarchies on the sidebar)
  //   Can do this later, though, functionally it's just another way to accomplish what we can do in the hierarchy.

  return (
    <div className="relative flex flex-1 flex-shrink-0">
      <label htmlFor="search" className="sr-only">
        Search
      </label>
      <input
        className="peer block w-full rounded-md border border-gray-200 py-[9px] pl-10 text-sm outline-2 placeholder:text-gray-500"
        placeholder={placeholder}
        onChange={(e) => {
          // eslint-disable-next-line @typescript-eslint/no-unsafe-call
          handleSearch(e.target.value)
        }}
        defaultValue={searchParams.get("query")?.toString()}
      />
    </div>
  )
}
