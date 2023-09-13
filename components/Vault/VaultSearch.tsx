import { ChangeEvent, useState, useRef } from "react";
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
  const query = useRef<HTMLDivElement>(null);

  const get_query = () => {
    const input = query.current?.querySelector("input");
    return input ? input.value : "";
  };

  const perform_search = async () => {
    try {
      const value = get_query();
      const results: GItem[] = await invoke("filter_by_name", { query: value });
      const items = results ? results : [];
      replace(items);
    } catch {}
  };

  return (
    <div className="relative flex w-full max-w-[24rem]" ref={query}>
      <Input
        type="text"
        onChange={perform_search}
        className="!border-pm-primary pr-20"
        crossOrigin={""}
        labelProps={{
          className: "before:content-none after:content-none",
        }}
        icon={<FontAwesomeIcon icon={faMagnifyingGlass} />}
      />
    </div>
  );
}
