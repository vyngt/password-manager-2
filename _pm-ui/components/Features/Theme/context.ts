"use client";

import type { ThemeState, ThemeAction } from "./define";
import type { Dispatch } from "react";
import { createContext } from "react";

export const initState: ThemeState = {
  colorSchemeId: 0,
};

export const initDispatch: Dispatch<ThemeAction> = () => {};

export const ThemeContext = createContext(initState);
export const ThemeDispatchContext = createContext(initDispatch);
