"use client";

import type { Dispatch, Context } from "react";
import type { TreeAction, DataModel } from "./define";
import { createContext } from "react";

export interface TreeState<T extends DataModel> {
  items: Array<T>;
  itemCount: number;
  currentPage: number;
  lastTerm: string;
}
export type TreeDispatch<T extends DataModel> = Dispatch<TreeAction<T>>;
export type TreeStateContext<T extends DataModel> = Context<TreeState<T>>;
export type TreeDispatchContext<T extends DataModel> = Context<TreeDispatch<T>>;

export function createTreeContext<T extends DataModel>({
  initState,
}: {
  initState: TreeState<T>;
}): [TreeStateContext<T>, TreeDispatchContext<T>] {
  const initDispatch: TreeDispatch<T> = () => {};
  const stateContext = createContext(initState);
  const dispatchContext = createContext(initDispatch);
  return [stateContext, dispatchContext];
}
