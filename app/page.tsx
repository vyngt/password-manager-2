"use client";
import { Typography } from "@material-tailwind/react";

import { Login, Register } from "@/components/Auth";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState, useContext } from "react";
import { useCurrentTheme } from "@/components/Theme";

export default function Home() {
  const [is_first_time, set_is_first_time] = useState(true);

  useCurrentTheme();

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
    <main className="flex h-full w-full flex-col items-center justify-center text-center">
      <Typography variant="h3">Password Manager</Typography>
      {is_first_time ? <Register /> : <Login />}
    </main>
  );
}
