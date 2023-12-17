"use client";

import "./style.css";
import { Card, Input, Button, Typography } from "@material-tailwind/react";
import { KeyboardEvent, MouseEvent, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useRouter } from "next/navigation";

export interface IBase {
  title: string;
  button: string;
  handler: "login" | "register";
  error_msg: string;
}

export default function Base({ base }: { base: IBase }) {
  const [password, set_password] = useState("");
  const [error, set_error] = useState(false);
  const [loading, set_loading] = useState(false);
  const router = useRouter();

  const call_server = async (password: string) => {
    try {
      const ok = await invoke(base.handler, { password: password });
      if (ok) {
        set_error(false);
        router.replace("/main");
      } else {
        set_error(true);
      }
    } catch (e) {
      set_error(true);
    }
  };

  const perform_call_server = async () => {
    if (password.length < 1) {
      return;
    }

    set_loading(true);
    await call_server(password);
    set_loading(false);
  };

  const handler_keypress = async (ev: KeyboardEvent<HTMLInputElement>) => {
    if (ev.key == "Enter") {
      ev.preventDefault();
      await perform_call_server();
    }
  };

  const perform_submit = async (e: MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    perform_call_server();
  };

  return (
    <Card color="transparent" shadow={false}>
      <Typography variant="lead" className="!text-pm-foreground">
        {base.title}
      </Typography>
      <form className="mb-2 mt-8 w-80 max-w-screen-lg sm:w-96">
        <div className="mb-4 flex flex-col gap-6">
          <Input
            type="password"
            size="lg"
            placeholder="Master Password"
            value={password}
            onChange={(e) => {
              set_password(e.target.value);
            }}
            onKeyDownCapture={handler_keypress}
            crossOrigin={""}
            labelProps={{
              className: "before:content-none after:content-none",
            }}
            containerProps={{
              className: "min-w-0",
            }}
            className="!border-pm-primary text-pm-foreground"
          />
          {error && (
            <Typography variant="small" className="!text-pm-danger">
              {base.error_msg}
            </Typography>
          )}
        </div>
        <Button
          className="mt-6 !bg-pm-primary !text-pm-foreground"
          fullWidth
          onClick={perform_submit}
          disabled={loading}
        >
          {base.button}
        </Button>
      </form>
    </Card>
  );
}
