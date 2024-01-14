"use client";

import { VaultBody } from "./VaultBody";
import { VaultHeader } from "./VaultHeader";

export function Vault() {
  return (
    <div className="flex h-full w-full flex-col">
      <VaultHeader />
      <VaultBody />
    </div>
  );
}
