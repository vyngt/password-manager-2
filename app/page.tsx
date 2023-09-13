"use client";
import { Typography } from "@material-tailwind/react";

import { Login, Register } from "@/components/Auth";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState, useContext } from "react";
import { useColorScheme, ThemeContext } from "@/components/Theme";
import { IColorScheme } from "@/components/Theme/types";

export default function Home() {
  const [is_first_time, set_is_first_time] = useState(true);
  const theme = useContext(ThemeContext);

  const { background, foreground } = useColorScheme();

  const check_if_is_first = () => {
    invoke("is_first_time")
      .then((e) => {
        set_is_first_time(e as boolean);
      })
      .catch(console.error);
  };

  const load_theme = async () => {
    const color_scheme: IColorScheme = await invoke("get_current_color_scheme");
    theme.set_color_scheme(color_scheme);
  };

  useEffect(() => {
    check_if_is_first();
  }, []);

  useEffect(() => {
    const load_ui_color = async () => {
      await load_theme();
    };
    load_ui_color();
  }, []);

  return (
    <main
      className="flex h-full w-full flex-col items-center justify-center text-center"
      style={{ backgroundColor: background }}
    >
      <Typography variant="h3" style={{ color: foreground }}>
        Password Manager
      </Typography>
      {is_first_time ? <Register /> : <Login />}
    </main>
  );
}
