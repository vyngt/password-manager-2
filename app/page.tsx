"use client";
import { Typography } from "@material-tailwind/react";

import { Login, Register } from "@/components/Auth";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { useColorScheme } from "@/components/Theme";

export default function Home() {
  const [is_first_time, set_is_first_time] = useState(true);

  const { background, foreground } = useColorScheme();

  const check_if_is_first = () => {
    invoke("is_first_time")
      .then((e) => {
        set_is_first_time(e as boolean);
      })
      .catch(console.error);
  };

  useEffect(() => {
    check_if_is_first();
  }, []);

  return (
    <main
      className="w-full h-full flex flex-col justify-center text-center items-center"
      style={{ backgroundColor: background }}
    >
      <Typography variant="h3" style={{ color: foreground }}>
        Password Manager
      </Typography>
      {is_first_time ? <Register /> : <Login />}
    </main>
  );
}
