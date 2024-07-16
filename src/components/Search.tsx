"use client"

import { usePathname, useRouter, useSearchParams } from "next/navigation"
import { useDebouncedCallback } from "use-debounce"

// TODO Transition to a React.FC component to get better type checking.

export default function Search({ placeholder }: { placeholder: string }) {
  const searchParams = useSearchParams()
  const pathname = usePathname()
  // eslint-disable-next-line @typescript-eslint/unbound-method
  const { replace } = useRouter()

  // TODO Well, this is the whole "Search" component.
  //   Right now, it just has the text input field that we put in ("query") (note: rename
  //       that to be something like search_text, since we'll have other query elements).
  //   We can add other input forms (such as a color picker, time range, tag selection (?) to this,
  //       and then create a new entry in params which does this.)
  //  TODO Note that I think we would also have a sidebar of hierarchical tags,
  //    and the URL search params.tags would be set there.
  //    That should be fine, since I think that the searchParams can effectively be used
  //    globally by two different components. We wouldn't want to do tags here then, though.
  // TODO In that case, maybe we DO keep this component as solely for the text search bar?
  //    Then they all just modify the searchParams object with the part of the query they care about.
  // TODO If we split search components, then page shouldn't be handled here, since it depends
  //    on all of them. But if we are switching to an infinite scroll anyways, then maybe don't
  //    worry about this yet... In fact, maybe just delete pagination logic until we need it.

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call
  const handleSearch = useDebouncedCallback((term) => {
    // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
    console.log(`Searching... ${term}`)
    const params = new URLSearchParams(searchParams)
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
