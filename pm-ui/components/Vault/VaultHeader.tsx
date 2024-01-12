"use client";

import { IconButton, Button } from "@/components/MaterialTailwind";
import { VaultPagination } from "./VaultPagination";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate } from "@fortawesome/free-solid-svg-icons";
import { VaultFilter } from "./VaultFilter";
import { useContext } from "react";
import { VaultDispatchContext } from "./contexts";
import { invoke } from "@tauri-apps/api/tauri";
import { Item, ResultWithCount } from "./define";
import Link from "next/link";

function VaultHeaderButton() {
  const dispatch = useContext(VaultDispatchContext);

  const reload = async () => {
    const data = await invoke<ResultWithCount<Item>>("fetch_items", {
      page: 1,
      term: "",
    });
    dispatch({ type: "load", payload: { data } });
  };

  return (
    <>
      <Link href="/main/vault/form#id=0">
        <Button variant="filled" className="bg-primary/70 text-foreground">
          New
        </Button>
      </Link>
      <IconButton
        onClick={reload}
        variant="outlined"
        className="border-secondary text-secondary focus:ring-secondary/30"
      >
        <FontAwesomeIcon icon={faArrowsRotate} />
      </IconButton>
    </>
  );
}

export function VaultHeader() {
  return (
    <div className="flex h-16 shrink-0 grow-0 border-b border-b-secondary bg-secondary/10">
      <div className="flex w-full flex-col justify-center">
        <div className="flex justify-around">
          <div className="flex gap-1 pl-4">
            <VaultHeaderButton />
          </div>
          <div className="flex grow justify-center px-2">
            <VaultFilter />
          </div>
          <div className="pr-4">
            <VaultPagination />
          </div>
        </div>
      </div>
    </div>
  );
}
