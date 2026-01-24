'use client';
import { Search } from 'lucide-react';
import { useSearchContext } from 'fumadocs-ui/contexts/search';

export function SearchToggle() {
  const { setOpenSearch } = useSearchContext();

  return (
    <button
      type="button"
      data-search-full=""
      className="my-auto inline-flex w-full max-w-sm items-center gap-2 text-sm text-fd-muted-foreground max-md:hidden"
      onClick={() => setOpenSearch(true)}
    >
      <Search className="size-4" />
      Search&#8230;
      <kbd className="ms-auto">Ctrl K</kbd>
    </button>
  );
}