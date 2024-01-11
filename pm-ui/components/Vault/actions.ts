import type { Item } from "./define";

interface LoadItemsAction {
  type: "load";
  payload: Array<Item>;
}

export type IVaultAction = LoadItemsAction;
