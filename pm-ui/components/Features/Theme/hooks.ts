"use client";

import type { ThemeState, ThemeAction } from "./define";
import type { Dispatch } from "react";
import { ThemeContext, ThemeDispatchContext } from "./context";
import { useContext } from "react";

export function useThemeDispatch() {
  return useContext(ThemeDispatchContext);
}

export function useThemeState() {
  return useContext(ThemeContext);
}

export function useTheme(): [ThemeState, Dispatch<ThemeAction>] {
  const dispatch = useContext(ThemeDispatchContext);
  const state = useContext(ThemeContext);

  return [state, dispatch];
}
