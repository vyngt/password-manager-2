"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { IconButton } from "../MaterialTailwind";
import { faBan } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/tauri";

import { useTree } from "./core";
import { FilterDataAction, TreeData } from "../UI/TreeView/define";
import { Item } from "./define";

export function DeleteButton({ id }: { id: number }) {
  const [state, dispatch] = useTree();

  const refetch = async ({
    term = "",
    page = 1,
  }: {
    term?: string;
    page?: number;
  }) => {
    return await invoke<TreeData<Item>>("fetch_items", { page, term });
  };

  const performAfterDelete = async () => {
    if (state.items.length > 0) {
      const payload: FilterDataAction<Item>["payload"] = {
        data: {
          result: [],
          total: 0,
        },
        searchTerm: state.lastTerm,
        page: state.currentPage,
      };

      if (state.items.length === 1) {
        const curr = state.currentPage;
        const prevPage = curr === 1 ? 1 : curr - 1;
        const data = await refetch({
          page: prevPage,
          term: state.lastTerm,
        });

        payload.data = data;
        payload.page = prevPage;
      } else {
        const data = await refetch({
          page: state.currentPage,
          term: state.lastTerm,
        });

        payload.data = data;
      }

      dispatch({ type: "filter", payload });
    } else {
      const data = await refetch({});
      dispatch({ type: "load", payload: { data } });
    }
  };

  const performDeleteItem = async () => {
    const result = await invoke<boolean>("delete_item", { id });
    if (result) await performAfterDelete();
  };

  return (
    <IconButton
      onClick={performDeleteItem}
      variant="text"
      className="text-danger hover:bg-primary/20 active:bg-primary/40"
    >
      <FontAwesomeIcon icon={faBan} />
    </IconButton>
  );
}
