"use client";

import { useReducer, useContext } from "react";

import type { DataModel } from "./define";
import type { TreeReducer } from "./reducer";
import type { TreeState, TreeDispatch } from "./context";
import { createTreeContext } from "./context";

export function createTreeProvider<T extends DataModel>({
  reducer,
}: {
  reducer: TreeReducer<T>;
}) {
  const initState: TreeState<T> = {
    items: [],
    itemCount: 0,
    currentPage: 0,
    lastTerm: "",
  };
  const [TreeContext, TreeDispatchContext] = createTreeContext({ initState });
  const Provider = ({ children }: { children: React.ReactNode }) => {
    const [state, dispatch] = useReducer(reducer, initState);
    return (
      <TreeContext.Provider value={state}>
        <TreeDispatchContext.Provider value={dispatch}>
          {children}
        </TreeDispatchContext.Provider>
      </TreeContext.Provider>
    );
  };

  const useTreeState = () => {
    return useContext(TreeContext);
  };

  const useTreeDispatch = () => {
    return useContext(TreeDispatchContext);
  };

  const useTree = (): [TreeState<T>, TreeDispatch<T>] => {
    const state = useContext(TreeContext);
    const dispatch = useContext(TreeDispatchContext);
    return [state, dispatch];
  };

  return {
    Provider: Provider,
    hook: {
      useTree,
      useTreeState,
      useTreeDispatch,
    },
  };
}
