"use client";

import { Typography } from "@/components/MaterialTailwind";
import { invoke } from "@tauri-apps/api/tauri";
import { useContext, useEffect, useCallback } from "react";
import { VaultContext, VaultDispatchContext } from "./contexts";
import { Item } from "./define";

const TABLE_HEAD = ["Name", "URL", "Username", ""];

export function VaultBody() {
  const state = useContext(VaultContext);
  const dispatch = useContext(VaultDispatchContext);

  const retrieveItems = useCallback(async () => {
    const data: Array<Item> = await invoke("fetch_items");
    dispatch({ type: "load", payload: data });
  }, [dispatch]);

  useEffect(() => {
    retrieveItems().finally(() => {});
  }, [retrieveItems]);

  return (
    <div className="grow">
      <table className="w-full min-w-max table-auto text-left">
        <thead>
          <tr className="border-b border-b-secondary/60">
            {TABLE_HEAD.map((head) => (
              <th key={head} className="p-4 capitalize">
                <Typography
                  variant="small"
                  className="flex items-center justify-between gap-2 font-bold leading-none"
                >
                  {head}
                </Typography>
              </th>
            ))}
          </tr>
        </thead>
        <tbody></tbody>
      </table>
    </div>
  );
}
