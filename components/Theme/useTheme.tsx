"use client";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect } from "react";

import { useContext } from "react";
import { ThemeContext } from "./ThemeProvider";
import { IColorScheme } from "./types";

export const useColorScheme = () => {
  const context = useContext(ThemeContext);
  return { ...context.color_scheme };
};

export const useColorSchemeManager = () => {
  const context = useContext(ThemeContext);
  return {
    change_to: context.change_to,
    set_color_scheme: context.set_color_scheme,
  };
};

export const useCurrentTheme = () => {
  const context = useContext(ThemeContext);

  useEffect(() => {
    const load_theme = async () => {
      const color_scheme: IColorScheme = await invoke(
        "get_current_color_scheme",
      );
      context.set_color_scheme(color_scheme);
    };
    load_theme();
  }, [context]);
};
