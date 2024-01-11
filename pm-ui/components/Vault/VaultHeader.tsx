"use client";

import { IconButton, Button } from "@/components/MaterialTailwind";
import { VaultPagination } from "./VaultPagination";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate } from "@fortawesome/free-solid-svg-icons";
import { VaultFilter } from "./VaultFilter";
import Link from "next/link";

function VaultHeaderButton() {
  return (
    <>
      <Link href="/main/vault/form#id=0">
        <Button variant="filled" className="bg-primary/70 text-foreground">
          New
        </Button>
      </Link>
      <IconButton
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
    <div className="flex h-16 grow-0 border-b border-b-secondary bg-secondary/10">
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
