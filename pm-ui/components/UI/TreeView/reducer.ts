"use client";

import type { DataModel, TreeAction } from "./define";
import type { TreeState } from "./context";

export type TreeReducer<T extends DataModel> = (
  state: TreeState<T>,
  action: TreeAction<T>,
) => TreeState<T>;

export function createTreeReducer<T extends DataModel>(): TreeReducer<T> {
  const reducer = (
    state: TreeState<T>,
    action: TreeAction<T>,
  ): TreeState<T> => {
    switch (action.type) {
      case "load":
        return {
          ...state,
          items: action.payload.data.result,
          itemCount: action.payload.data.total,
          currentPage: 1,
          lastTerm: "",
        };
      case "filter":
        return {
          ...state,
          items: action.payload.data.result,
          lastTerm: action.payload.searchTerm,
          itemCount: action.payload.data.total,
          currentPage: action.payload.page ? action.payload.page : 1,
        };
      case "paginate":
        return {
          ...state,
          items: action.payload.data.result,
          currentPage: action.payload.nextPage,
        };
    }
  };

  return reducer;
}
