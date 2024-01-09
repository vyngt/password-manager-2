"use client";

import { createContext, useReducer } from "react";

interface IVaultContext {}
interface IVaultDispatchContext {}

export const VaultContext = createContext<IVaultContext>({});
export const VaultDispatchContext = createContext<IVaultDispatchContext>({});

export const VaultProvider = ({ children }: { children: React.ReactNode }) => {
  const context: IVaultContext = {};
  const dispatch: IVaultDispatchContext = {};

  return (
    <VaultContext.Provider value={context}>
      <VaultDispatchContext.Provider value={dispatch}>
        {children}
      </VaultDispatchContext.Provider>
    </VaultContext.Provider>
  );
};
