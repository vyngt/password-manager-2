"use client";

import { useRouter } from "next/navigation";
import { Typography } from "@/components/MaterialTailwind";
import { invoke } from "@tauri-apps/api/tauri";
import { useContext, useEffect, useCallback } from "react";
import { VaultContext, VaultDispatchContext } from "./contexts";
import { IconButton } from "@/components/MaterialTailwind";
import { Item } from "./define";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faKey, faBan } from "@fortawesome/free-solid-svg-icons";

const TABLE_HEAD = ["Name", "URL", "Username", ""];

function Row({ item, even }: { item: Item; even?: boolean }) {
  const dispatch = useContext(VaultDispatchContext);

  const reload = async () => {
    const data: Array<Item> = await invoke("fetch_items");
    dispatch({ type: "load", payload: data });
  };

  const router = useRouter();
  const navigate = () => {
    router.push(`/main/vault/form#id=${item.id}`);
  };

  const performCopyKeyToClipboard = async () => {
    const result: string = await invoke("get_item_key", { id: item.id });
    navigator.clipboard.writeText(result);
  };

  const performDeleteItem = async () => {
    const result: boolean = await invoke("delete_item", { id: item.id });
    if (result) await reload();
  };

  return (
    <tr
      className={`w-full cursor-pointer border-b border-b-secondary/60 text-secondary transition-colors hover:bg-primary/30 ${
        even ? "bg-primary/10" : ""
      }`}
    >
      <td className="py-2" onClick={navigate}>
        <div className="flex items-center gap-3">
          <div className="flex flex-col">
            <Typography variant="small" className="pl-4 font-normal">
              {item.name}
            </Typography>
          </div>
        </div>
      </td>
      <td className="py-2" onClick={navigate}>
        <div className="flex flex-col">
          <Typography variant="small" className="pl-4 font-normal">
            {item.url}
          </Typography>
        </div>
      </td>
      <td className="py-2" onClick={navigate}>
        <Typography variant="small" className="pl-4 font-normal">
          {item.username}
        </Typography>
      </td>
      <td className="py-2">
        <IconButton
          onClick={performCopyKeyToClipboard}
          variant="text"
          className="text-secondary hover:bg-primary/20 active:bg-primary/40"
        >
          <FontAwesomeIcon icon={faKey} />
        </IconButton>
        <IconButton
          onClick={performDeleteItem}
          variant="text"
          className="text-danger hover:bg-primary/20 active:bg-primary/40"
        >
          <FontAwesomeIcon icon={faBan} />
        </IconButton>
      </td>
    </tr>
  );
}

export function VaultBody() {
  const state = useContext(VaultContext);
  const dispatch = useContext(VaultDispatchContext);

  const retrieveItems = useCallback(async () => {
    const data: Array<Item> = await invoke("fetch_items");
    return data;
  }, []);

  useEffect(() => {
    retrieveItems().then((data) => {
      dispatch({ type: "load", payload: data });
    });
  }, [retrieveItems]);

  return (
    <div className="grow">
      <table className="w-full min-w-max table-auto text-left">
        <thead>
          <tr className="border-b border-b-secondary/75 bg-primary/10 text-primary">
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
        <tbody>
          {state.items.map((item, idx) => (
            <Row key={item.id} even={idx % 2 != 0} item={item} />
          ))}
        </tbody>
      </table>
    </div>
  );
}
