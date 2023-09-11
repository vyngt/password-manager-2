"use client";

import { createContext, useState } from "react";
import { IColorScheme, IThemeContextProps } from "./types";

const default_color_scheme: IColorScheme = {
  id: 0,
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
});

export const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  const [theme, setTheme] = useState(default_color_scheme);

  const context_props: IThemeContextProps = {
    color_scheme: theme,
    change_to: async (id: number) => {
      // Perform Fetch ID
      // Set Theme
    },
  };

  return (
    <ThemeContext.Provider value={context_props}>
      {children}
    </ThemeContext.Provider>
  );
};
