"use client";

import { useContext } from "react";
import { ThemeContext } from "./ThemeProvider";

export const useColorScheme = () => {
  const context = useContext(ThemeContext);
  return { ...context.color_scheme };
};
