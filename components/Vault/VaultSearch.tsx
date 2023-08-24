import { useEffect, useState } from "react";
import { Input, Button } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/tauri";
import { GItem } from "./models";

export function VaultSearch({
  replace,
}: {
  replace: (items: GItem[]) => void;
}) {
  const [query, setQuery] = useState("");

  useEffect(() => {
    const callback_search = setTimeout(async () => {
      await perform_search();
    }, 150);
    return () => clearTimeout(callback_search);
  }, [query]);

  const perform_search = async () => {
    try {
      const results: GItem[] = await invoke("filter_by_name", { query: query });
      const items = results ? results : [];
      replace(items);
    } catch {}
  };

  return (
    <div className="relative flex w-full max-w-[24rem]">
      <Input
        type="text"
        label="Keyword"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="pr-20"
        crossOrigin={""}
      />
      <Button
        size="sm"
        color={query ? "gray" : "blue-gray"}
        disabled={!query}
        className="!absolute right-1 top-1 rounded"
        onClick={perform_search}
      >
        <FontAwesomeIcon icon={faMagnifyingGlass} />
      </Button>
    </div>
  );
}
