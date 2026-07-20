"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { IconButton } from "@/components/MaterialTailwind";
import { faBan } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/tauri";

import type { FilterDataAction, TreeData, DataModel } from "../define";
import type { TreeState, TreeDispatch } from "../context";

// Code horrible, TODO: Reform
export function DeleteButton<T extends DataModel>({
  id,
  refetchId,
  deleteId,
  state,
  dispatch,
}: {
  id: number;
  refetchId: string;
  deleteId: string;
  state: TreeState<T>;
  dispatch: TreeDispatch<T>;
}) {
  const refetch = async ({
    term = "",
    page = 1,
  }: {
    term?: string;
    page?: number;
  }) => {
    return await invoke<TreeData<T>>(refetchId, { page, term });
  };

  const performAfterDelete = async () => {
    if (state.items.length > 0) {
      const payload: FilterDataAction<T>["payload"] = {
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
    const result = await invoke<boolean>(deleteId, { id });
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
