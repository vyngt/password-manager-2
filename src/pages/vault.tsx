import { VTable } from "@/components/Vault";
import { Layout } from "@/components/Layout";
import { GItem, Item, NewItem, Operator } from "@/models";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState, useRef } from "react";

const init_edit_item: Item = {
  id: 0,
  name: "",
  url: "",
  username: "",
  password: "",
};

export default function Vault() {
  const [items, set_items] = useState<GItem[]>([]);
  const [edit_item, set_edit_item] = useState<Item>(init_edit_item);
  const block = useRef<HTMLDivElement>(null);

  useEffect(() => {
    async function init_data() {
      const response_data: GItem[] = await invoke("fetch_all_items");
      set_items(response_data);
    }
    init_data();
  }, []);

  const ItemCRUD = () => {
    const copy = (item: GItem) => {
      console.log("Call Copy");
    };

    const remove_item = (item: GItem) => {
      let new_arr = [...items];
      const index = items.findIndex((i) => i.id == item.id, item);
      new_arr.splice(index, 1);
      set_items(new_arr);
    };

    const update_item = (item: GItem) => {
      console.log("Call Update");
      // Hide current block
      block?.current?.classList?.add("hide");

      // DIsplay Edit Form block
    };

    const operator: Operator = {
      copy: copy,
      delete: remove_item,
      update: update_item,
    };
    return operator;
  };

  return (
    <Layout>
      <>
        <VTable ref={block} items={items} operator={ItemCRUD()} />
      </>
    </Layout>
  );
}
