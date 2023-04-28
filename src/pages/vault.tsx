import { VTable, VEditItemForm } from "@/components/Vault";
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
  const vtable = useRef<HTMLDivElement>(null);
  const veditform = useRef<HTMLDivElement>(null);
  // const

  useEffect(() => {
    async function init_data() {
      const response_data: GItem[] = await invoke("fetch_all_items");
      set_items(response_data);
    }
    init_data();
  }, []);

  const ItemCRUD = () => {
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
      invoke("fetch_item", { id: item.id })
        .then((e) => {
          set_edit_item(e as Item);
        })
        .catch(console.error);
      block_operator.to_edit();
    };

    const operator: Operator<GItem> = {
      copy: copy,
      delete: remove_item,
      update: update_item,
    };
    return operator;
  };

  const block_operator = {
    to_edit: () => {
      vtable?.current?.classList?.add("hide");
      vtable?.current?.classList?.remove("show");
      veditform?.current?.classList?.remove("hide");
      veditform?.current?.classList?.add("show");
    },
    to_table: () => {
      veditform?.current?.classList?.add("hide");
      veditform?.current?.classList?.remove("show");
      vtable?.current?.classList?.remove("hide");
      vtable?.current?.classList?.add("show");
    },
  };

  return (
    <Layout>
      <>
        <VTable ref={vtable} items={items} operator={ItemCRUD()} />
        <VEditItemForm
          ref={veditform}
          item={edit_item}
          close_handler={block_operator.to_table}
        />
      </>
    </Layout>
  );
}
