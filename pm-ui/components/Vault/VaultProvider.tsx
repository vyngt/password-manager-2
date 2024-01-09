"use client";

import { useReducer } from "react";
import { VaultContext, VaultDispatchContext, initContext } from "./contexts";
import reducer from "./reducers";

export const VaultProvider = ({ children }: { children: React.ReactNode }) => {
  const [state, dispatch] = useReducer(reducer, initContext);
  return (
    <VaultContext.Provider value={state}>
      <VaultDispatchContext.Provider value={dispatch}>
        {children}
      </VaultDispatchContext.Provider>
    </VaultContext.Provider>
  );
};
