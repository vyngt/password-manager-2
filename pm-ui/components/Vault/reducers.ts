"use client";

import type { IVaultAction } from "./actions";
import type { IVaultContext } from "./contexts";

export default function vaultReducer(
  state: IVaultContext,
  action: IVaultAction,
): IVaultContext {
  switch (action.type) {
    case "load":
      return { ...state, items: action.payload };
  }
}
