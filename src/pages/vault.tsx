import { VTable } from "@/components/Vault";
import { Layout } from "@/components/Layout";
import { GItem, NewItem, Operator } from "@/models";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState, useRef } from "react";

export default function Vault() {
  const [items, set_items] = useState<GItem[]>([]);
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
      if (block !== null) {
        console.log(block.current?.className);
      }
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
      <div ref={block} className="vynt">
        <VTable items={items} operator={ItemCRUD()} />
      </div>
    </Layout>
  );
}
