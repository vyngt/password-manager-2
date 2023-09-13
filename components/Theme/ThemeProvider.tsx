"use client";

import { invoke } from "@tauri-apps/api/tauri";
import { createContext, useState } from "react";
import { IColorScheme, IThemeContextProps } from "./types";

const default_color_scheme: IColorScheme = {
  id: 0,
  name: "Default",
  primary: "blue",
  secondary: "gray",
  success: "green",
  danger: "red",
  warning: "yellow",
  foreground: "#ffffff",
  background: "#000000",
  selection: "white",
};

export const ThemeContext = createContext<IThemeContextProps>({
  color_scheme: default_color_scheme,
  change_to: () => {},
  set_color_scheme() {},
});

export const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  const [theme, setTheme] = useState(default_color_scheme);

  const apply_root_color_scheme = (css_color_scheme: IColorScheme) => {
    const root = document.documentElement;

    for (const [key, value] of Object.entries(css_color_scheme)) {
      const is_not_valid = key == "name" || key == "id";
      if (!is_not_valid) {
        root.style.setProperty(`--${key}`, value);
      }
    }
  };

  const context_props: IThemeContextProps = {
    color_scheme: theme,
    change_to: async (id: number) => {
      const ok: true = await invoke("save_theme", { id });
      if (ok) {
        const scheme: IColorScheme | undefined = await invoke(
          "get_current_color_scheme",
        );
        if (scheme) {
          apply_root_color_scheme(scheme);
          setTheme(scheme);
        }
      }
    },
    set_color_scheme: (scheme) => {
      apply_root_color_scheme(scheme);
      setTheme(scheme);
    },
  };

  return (
    <ThemeContext.Provider value={context_props}>
      {children}
    </ThemeContext.Provider>
  );
};
