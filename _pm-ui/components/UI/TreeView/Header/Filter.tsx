"use client";

import { Input } from "@/components/MaterialTailwind";
import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useEffect, useState } from "react";

export function SearchFilter({
  handleQuery,
}: {
  handleQuery: (q: string) => void;
}) {
  const [query, setQuery] = useState<string | null>(null);

  // Debounce
  useEffect(() => {
    if (query === null) return;
    const cb = setTimeout(() => {
      handleQuery(query.trim());
    }, 150);
    return () => clearTimeout(cb);
  }, [query, handleQuery]);

  return (
    <div className="w-1/2">
      <Input
        icon={
          <FontAwesomeIcon
            icon={faMagnifyingGlass}
            className="text-secondary/60"
          />
        }
        crossOrigin={""}
        type="text"
        placeholder="Search"
        onChange={(ev) => setQuery(ev.target.value)}
        className="!border-secondary/60 text-foreground placeholder:text-foreground/60 focus:!border-secondary"
        labelProps={{
          className: "before:content-none after:content-none",
        }}
        containerProps={{
          className: "min-w-0",
        }}
      />
    </div>
  );
}
