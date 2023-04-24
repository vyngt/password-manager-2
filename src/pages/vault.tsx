import { VTable } from "@/components/Vault";
import { Layout } from "@/components/Layout";
import { GItem } from "@/models";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";

export default function Vault() {
  let [items, set_items] = useState<GItem[]>([]);

  useEffect(() => {
    invoke("fetch_all_items").then((e) => {
      let g_items = e as GItem[];
      set_items(g_items);
    });
  }, []);

  useEffect(() => {}, [items]);

  return (
    <Layout>
      <VTable items={items} />
    </Layout>
  );
}
