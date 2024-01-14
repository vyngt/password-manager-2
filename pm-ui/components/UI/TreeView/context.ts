"use client";

import type { Dispatch, Context } from "react";
import type { TreeAction, Data } from "./define";
import { createContext } from "react";

export interface TreeState<T extends Data> {
  items: Array<T>;
  itemCount: number;
  currentPage: number;
  lastTerm: string;
}
export type TreeDispatch<T extends Data> = Dispatch<TreeAction<T>>;
export type TreeStateContext<T extends Data> = Context<TreeState<T>>;
export type TreeDispatchContext<T extends Data> = Context<TreeDispatch<T>>;

export function createTreeContext<T extends Data>({
  initState,
}: {
  initState: TreeState<T>;
}): [TreeStateContext<T>, TreeDispatchContext<T>] {
  const initDispatch: TreeDispatch<T> = () => {};
  const stateContext = createContext(initState);
  const dispatchContext = createContext(initDispatch);
  return [stateContext, dispatchContext];
}
