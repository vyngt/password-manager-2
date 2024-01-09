"use client";

import type { Dispatch } from "react";
import type { Item } from "./define";
import type { IVaultAction } from "./actions";
import { createContext } from "react";

export interface IVaultContext {
  items: Array<Item>;
}

export type IVaultDispatchContext = Dispatch<IVaultAction>;

export const initContext: IVaultContext = {
  items: [],
};

const initDispatch: IVaultDispatchContext = () => {};

export const VaultContext = createContext(initContext);
export const VaultDispatchContext = createContext(initDispatch);
