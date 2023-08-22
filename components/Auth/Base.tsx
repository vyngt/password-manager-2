"use client";

import { Card, Input, Button, Typography } from "@material-tailwind/react";
import { MouseEvent, useState } from "react";
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
  const router = useRouter();

  const call_server = (password: string) => {
    invoke(base.handler, { password: password })
      .then((e) => {
        if (e) {
          set_error(false);
          router.replace("/main");
        } else {
          set_error(true);
        }
      })
      .catch(() => {
        set_error(true);
      });
  };

  const perform_submit = (e: MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    if (password.length < 1) {
      return;
    }

    e.currentTarget.disabled = true;
    call_server(password);
    e.currentTarget.disabled = false;
  };

  return (
    <Card color="transparent" shadow={false}>
      <Typography variant="h4" color="blue-gray">
        {base.title}
      </Typography>
      <form className="mt-8 mb-2 w-80 max-w-screen-lg sm:w-96">
        <div className="mb-4 flex flex-col gap-6">
          <Input
            type="password"
            size="lg"
            label="Master Password"
            value={password}
            onChange={(e) => {
              set_password(e.target.value);
            }}
            crossOrigin={""}
          />
          {error ? (
            <Typography variant="small" color="red">
              {base.error_msg}
            </Typography>
          ) : (
            ""
          )}
        </div>
        <Button className="mt-6" fullWidth onClick={perform_submit}>
          {base.button}
        </Button>
      </form>
    </Card>
  );
}
