import Head from "next/head";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { Login, Register } from "./components/Auth";

export default function Home() {
  const [is_first_time, set_is_first_time] = useState(true);

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
    <>
      <Head>
        <title>Password Manager</title>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
      </Head>
      <main>
        <h3>Password Manager</h3>
        {is_first_time ? <Register /> : <Login />}
      </main>
    </>
  );
}
