"use client";

import { useHashParam } from "@/lib/utils";
import { useEffect } from "react";
import { Input } from "@/components/UI";

export default function VaultItemForm() {
  const p = useHashParam();

  useEffect(() => {
    console.log(p.get("id"));
  }, []);

  return (
    <div className="p-4">
      <form className="flex flex-col gap-2">
        <Input color="primary" label="Name" />
        <Input color="primary" label="URL" />
        <Input color="primary" label="Username" />
        <Input color="primary" label="Password" type="password" />
      </form>
    </div>
  );
}
