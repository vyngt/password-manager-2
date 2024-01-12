"use client";

import { Input } from "@/components/MaterialTailwind";
import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useEffect, useState, useCallback, useContext } from "react";
import { VaultDispatchContext } from "./contexts";
import { Item } from "./define";
import { invoke } from "@tauri-apps/api/tauri";

export function VaultFilter() {
  const dispatch = useContext(VaultDispatchContext);
  const [query, setQuery] = useState<string | null>(null);

  const performQuery = useCallback(async (q: string) => {
    const param = { method: "fetch_items", args: {} };
    if (q.length > 0) {
      (param.method = "query_items"), (param.args = { query: q });
    }
    const result = await invoke<Array<Item>>(param.method, param.args);
    dispatch({ type: "load", payload: result });
  }, []);

  // Debounce
  useEffect(() => {
    if (query === null) return;
    const cb = setTimeout(() => {
      performQuery(query.trim());
    }, 150);
    return () => clearTimeout(cb);
  }, [query, performQuery]);

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
        className="!border-secondary/60 text-foreground focus:!border-secondary"
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
