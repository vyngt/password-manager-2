"use client";

import type { Dispatch } from "react";
import type { Item } from "./define";
import type { IVaultAction } from "./actions";
import { createContext } from "react";

export interface IVaultContext {
  items: Array<Item>;
  itemCount: number;
  currentPage: number;
  lastTerm: string;
}

export type IVaultDispatchContext = Dispatch<IVaultAction>;

export const initContext: IVaultContext = {
  items: [],
  itemCount: 0,
  currentPage: 1,
  lastTerm: "",
};

const initDispatch: IVaultDispatchContext = () => {};

export const VaultContext = createContext(initContext);
export const VaultDispatchContext = createContext(initDispatch);
