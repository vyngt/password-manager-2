"use client";

import { useContext } from "react";
import { VaultContext, VaultDispatchContext } from "./contexts";

export function useVaultState() {
  const state = useContext(VaultContext);
  return state;
}

export function useVaultDispatch() {
  const dispatch = useContext(VaultDispatchContext);
  return dispatch;
}

export function useVault() {
  const state = useContext(VaultContext);
  const dispatch = useContext(VaultDispatchContext);
  return { state, dispatch };
}
