"use client";

import { Card, Input, Button, Typography } from "@material-tailwind/react";
import { MouseEvent, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useRouter } from "next/navigation";
import { useColorScheme } from "../Theme";

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

  const { foreground, danger, primary, background, secondary } =
    useColorScheme();

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

  const perform_submit = async (e: MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    if (password.length < 1) {
      return;
    }

    set_loading(true);
    await call_server(password);
    set_loading(false);
  };

  return (
    <Card color="transparent" shadow={false}>
      <Typography variant="lead" style={{ color: foreground }}>
        {base.title}
      </Typography>
      <form className="mt-8 mb-2 w-80 max-w-screen-lg sm:w-96">
        <div className="mb-4 flex flex-col gap-6">
          <Input
            type="password"
            size="lg"
            label="Master Password"
            placeholder="Master Password"
            value={password}
            onChange={(e) => {
              set_password(e.target.value);
            }}
            // labelProps={{ className: "hidden" }}
            crossOrigin={""}
          />
          {error ? (
            <Typography variant="small" style={{ color: danger }}>
              {base.error_msg}
            </Typography>
          ) : (
            ""
          )}
        </div>
        <Button
          className="mt-6"
          fullWidth
          onClick={perform_submit}
          disabled={loading}
          style={{ backgroundColor: primary, color: foreground }}
        >
          {base.button}
        </Button>
      </form>
    </Card>
  );
}
