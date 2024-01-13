"use client";

import type { IVaultAction } from "./actions";
import type { IVaultContext } from "./contexts";

export default function vaultReducer(
  state: IVaultContext,
  action: IVaultAction,
): IVaultContext {
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
      };
    case "paginate":
      return {
        ...state,
        items: action.payload.data.result,
        currentPage: action.payload.nextPage,
      };
  }
}
