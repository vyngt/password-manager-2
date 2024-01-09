"use client";

import { Input } from "@/components/MaterialTailwind";
import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

export function VaultFilter() {
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
