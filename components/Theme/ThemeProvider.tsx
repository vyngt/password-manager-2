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
};

export const ThemeContext = createContext<IThemeContextProps>({
  color_scheme: default_color_scheme,
  change_to: () => {},
  set_color_scheme() {},
});

export const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  const [theme, setTheme] = useState(default_color_scheme);

  const context_props: IThemeContextProps = {
    color_scheme: theme,
    change_to: async (id: number) => {
      const ok: true = await invoke("save_theme", { id });
      if (ok) {
        const scheme: IColorScheme | undefined = await invoke(
          "get_current_color_scheme"
        );
        if (scheme) {
          setTheme(scheme);
        }
      }
    },
    set_color_scheme: (scheme) => setTheme(scheme),
  };

  return (
    <ThemeContext.Provider value={context_props}>
      {children}
    </ThemeContext.Provider>
  );
};
