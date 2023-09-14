"use client";

import { invoke } from "@tauri-apps/api/tauri";

import { FormEvent, useState } from "react";
import { Checkbox, Typography, Button, Input } from "@material-tailwind/react";

import { IPasswordGenerator } from "./models";

export default function PasswordGenerator() {
  const [state, setState] = useState<IPasswordGenerator>({
    len: 30,
    upper: true,
    lower: true,
    digits: true,
    special: true,
  });

  const UpdateState = {
    upper: () => {
      setState({ ...state, upper: !state.upper });
    },
    lower: () => {
      setState({ ...state, lower: !state.lower });
    },
    digits: () => {
      setState({ ...state, digits: !state.digits });
    },
    special: () => {
      setState({ ...state, special: !state.special });
    },
    len: (ev: FormEvent<HTMLInputElement>) => {
      let n = Number(ev.currentTarget.value);

      n = n > 300 ? 300 : n;
      n = n < 0 ? 0 : n;

      setState({ ...state, len: n });
    },
  };

  const [password, setPassword] = useState("");

  const perform_generate = async () => {
    try {
      const pw: string = await invoke("generate_password", { ...state });
      setPassword(pw);
    } catch (e) {}
  };

  return (
    <div className="flex flex-col justify-center bg-transparent p-2">
      <div className="mb-8 flex flex-col items-center justify-between gap-8">
        <Typography variant="h4" className="!text-pm-primary">
          Password Generator
        </Typography>
        <Typography className="mt-1 font-normal !text-pm-foreground">
          Let generate your strong password
        </Typography>
      </div>
      <div className="flex justify-center">
        <form className="mb-2 mt-8 flex flex-col items-center justify-center gap-2">
          <div className="relative h-10 w-full min-w-[200px]">
            <input
              className="pm-input peer text-pm-foreground"
              placeholder=" "
              value={state.len}
              type="number"
              onInput={UpdateState.len}
            />
            <label className="before:content[' '] after:content[' '] pm-input-label">
              Length
            </label>
          </div>
          <div className="flex gap-4">
            <Checkbox
              crossOrigin={""}
              className="!border-pm-primary bg-transparent checked:!border-transparent checked:bg-pm-primary"
              checked={state.lower}
              onChange={UpdateState.lower}
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal !text-pm-foreground"
                >
                  Lowercase
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
            <Checkbox
              className="!border-pm-primary bg-transparent checked:!border-transparent checked:bg-pm-primary"
              crossOrigin={""}
              checked={state.upper}
              onChange={UpdateState.upper}
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal !text-pm-foreground"
                >
                  Uppercase
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
            <Checkbox
              className="!border-pm-primary bg-transparent checked:!border-transparent checked:bg-pm-primary"
              crossOrigin={""}
              checked={state.digits}
              onChange={UpdateState.digits}
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal !text-pm-foreground"
                >
                  Digits
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
            <Checkbox
              className="!border-pm-primary bg-transparent checked:!border-transparent checked:bg-pm-primary"
              crossOrigin={""}
              checked={state.special}
              onChange={UpdateState.special}
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal !text-pm-foreground"
                >
                  Special
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
          </div>
          <div className="flex w-full justify-between">
            <Button
              className="mt-6 !bg-pm-primary !text-pm-foreground"
              onClick={perform_generate}
            >
              Generate
            </Button>
            <Button
              variant="outlined"
              className="mt-6 !border-pm-primary !text-pm-foreground"
              onClick={() => {
                navigator.clipboard.writeText(password);
              }}
            >
              Copy
            </Button>
          </div>
        </form>
      </div>
      {password && (
        <div className="mt-6 flex flex-col items-center rounded-lg border border-pm-foreground p-2">
          <div className="overflow-clip">
            <Typography className="!text-pm-foreground">{password}</Typography>
          </div>
        </div>
      )}
    </div>
  );
}
