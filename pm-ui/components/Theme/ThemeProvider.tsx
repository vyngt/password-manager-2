"use client";

import { invoke } from "@tauri-apps/api/tauri";
import type { ColorScheme as _ColorScheme } from "../ColorScheme/define";
import { useReducer, useEffect, useCallback } from "react";

import { ThemeReducer } from "./reducer";
import { ThemeContext, ThemeDispatchContext, initState } from "./context";

interface ColorScheme extends Omit<_ColorScheme, "id" | "name"> {}

export const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  const [state, dispatch] = useReducer(ThemeReducer, initState);

  const applyColorScheme = useCallback(async (id: number) => {
    const colorScheme = await invoke<ColorScheme>("get_theme_cs", { id });
    const root = document.documentElement;

    for (const [key, value] of Object.entries(colorScheme)) {
      root.style.setProperty(`--${key}`, value as string);
    }
  }, []);

  const getCurrentColorScheme = useCallback(async () => {
    // Fetch on Theme
    return await invoke<number>("get_current_cs");
  }, []);

  // Initialize
  useEffect(() => {
    getCurrentColorScheme().then((id) => {
      dispatch({ type: "color_scheme/set", payload: id });
    });
  }, [dispatch, getCurrentColorScheme]);

  // ChangeColor if change ID
  useEffect(() => {
    if (state.colorSchemeId > 0) {
      applyColorScheme(state.colorSchemeId);
    }
  }, [state.colorSchemeId, applyColorScheme]);

  return (
    <ThemeContext.Provider value={state}>
      <ThemeDispatchContext.Provider value={dispatch}>
        {children}
      </ThemeDispatchContext.Provider>
    </ThemeContext.Provider>
  );
};
