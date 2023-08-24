"use client";

import VaultTable from "./VaultTable";
import VaultPaginator from "./VaultPaginator";
import VaultHeader from "./VaultHeader";

import { GItem, IVaultHeaderManager, Item, Operator } from "./models";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";

export default function Vault() {
  const [items, set_items] = useState<GItem[]>([]);

  useEffect(() => {
    perform_retrieve_items();
  }, []);

  const perform_retrieve_items = async () => {
    const response_data: GItem[] = await invoke("fetch_all_items");
    set_items(response_data);
  };

  const VaultHeaderManager: IVaultHeaderManager<GItem> = {
    refresh: perform_retrieve_items,
    replace(items) {
      set_items(items);
    },
  };

  const ItemOperator = () => {
    const copy = (item: GItem) => {
      invoke("fetch_item", { id: item.id })
        .then((_res) => {
          const res = _res as Item;
          navigator.clipboard.writeText(res.password);
        })
        .catch(() => {
          navigator.clipboard.writeText("");
        });
    };

    const remove_item = (item: GItem) => {
      let new_arr = [...items];
      const id = item.id;
      invoke("delete_item", { id: id })
        .then(() => {
          const index = items.findIndex((i) => i.id == id, item);
          new_arr.splice(index, 1);
          set_items(new_arr);
        })
        .catch(() => {});
    };

    const update_item = (item: GItem) => {
      const new_arr = [...items];
      const id = item.id;
      const index = new_arr.findIndex((i) => i.id == id, item);
      new_arr[index] = item;
      set_items(new_arr);
    };

    const operator: Operator<GItem> = {
      copy: copy,
      delete: remove_item,
      update: update_item,
    };
    return operator;
  };

  return (
    <div className="relative flex flex-col bg-clip-border rounded-xl bg-white text-gray-700 shadow-md h-full w-full gap-5">
      <div className="relative bg-clip-border mt-4 mx-4 bg-white text-gray-700 rounded-none">
        <VaultHeader manager={VaultHeaderManager} />
      </div>
      <div className="p-6 overflow-x-scroll px-0 vault-table-scrollbar">
        <VaultTable items={items} operator={ItemOperator()} />
      </div>
      {/* <div className="flex items-center justify-between border-t border-blue-gray-50 p-4">
        <VaultPaginator />
      </div> */}
    </div>
  );
}
