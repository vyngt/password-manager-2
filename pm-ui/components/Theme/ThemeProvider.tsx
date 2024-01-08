"use client";

import { invoke } from "@tauri-apps/api/tauri";
import { createContext, useState } from "react";

interface IThemeContextProps {}

export const ThemeContext = createContext<IThemeContextProps>({});

export const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  const context_props: IThemeContextProps = {};

  return (
    <ThemeContext.Provider value={context_props}>
      {children}
    </ThemeContext.Provider>
  );
};
