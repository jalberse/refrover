"use client"

import { usePathname, useRouter, useSearchParams } from "next/navigation"
import { useDebouncedCallback } from "use-debounce"

// TODO Transition to a React.FC component to get better type checking.

export default function Search({ placeholder }: { placeholder: string }) {
  const searchParams = useSearchParams()
  const pathname = usePathname()
  // eslint-disable-next-line @typescript-eslint/unbound-method
  const { replace } = useRouter()

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call
  const handleSearch = useDebouncedCallback((term) => {
    // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
    console.log(`Searching... ${term}`)
    const params = new URLSearchParams(searchParams)
    params.set("page", "1")
    if (term) {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      params.set("query", term)
    } else {
      params.delete("query")
    }
    replace(`${pathname}?${params.toString()}`)
  }, 300)

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
