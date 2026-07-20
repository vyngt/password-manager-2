"use client";

import type { PasswordConfiguration } from "./define";
import { PGReducer } from "./define";
import { invoke } from "@tauri-apps/api/tauri";

import { useReducer, useState } from "react";

import { Checkbox, Typography, Button } from "@/components/MaterialTailwind";

const initConfiguration: PasswordConfiguration = {
  len: 30,
  upper: true,
  lower: true,
  digits: true,
  special: true,
};

export function PasswordGenerator() {
  const [state, dispatch] = useReducer(PGReducer, initConfiguration);

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
        <Typography variant="h4" className="text-primary">
          Password Generator
        </Typography>
        <Typography className="mt-1 font-normal text-foreground">
          Let generate your strong password
        </Typography>
      </div>
      <div className="flex justify-center">
        <form className="mb-2 mt-8 flex flex-col items-center justify-center gap-2">
          <div>{state.len}</div>
          <div className="relative h-10 w-full min-w-[200px]">
            <input
              className="w-full accent-primary"
              value={state.len}
              type="range"
              min={1}
              max={300}
              onChange={(ev) =>
                dispatch({ type: "len", payload: Number(ev.target.value) })
              }
            />
          </div>
          <div className="flex gap-4">
            <Checkbox
              crossOrigin={""}
              className="!border-primary bg-transparent checked:!border-transparent checked:bg-primary"
              checked={state.lower}
              onChange={() =>
                dispatch({
                  type: "other",
                  payload: { key: "lower", value: !state.lower },
                })
              }
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal text-foreground"
                >
                  Lowercase
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
            <Checkbox
              className="!border-primary bg-transparent checked:!border-transparent checked:bg-primary"
              crossOrigin={""}
              checked={state.upper}
              onChange={() =>
                dispatch({
                  type: "other",
                  payload: { key: "upper", value: !state.upper },
                })
              }
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal text-foreground"
                >
                  Uppercase
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
            <Checkbox
              className="!border-primary bg-transparent checked:!border-transparent checked:bg-primary"
              crossOrigin={""}
              checked={state.digits}
              onChange={() =>
                dispatch({
                  type: "other",
                  payload: { key: "digits", value: !state.digits },
                })
              }
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal text-foreground"
                >
                  Digits
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
            <Checkbox
              className="!border-primary bg-transparent checked:!border-transparent checked:bg-primary"
              crossOrigin={""}
              checked={state.special}
              onChange={() =>
                dispatch({
                  type: "other",
                  payload: { key: "special", value: !state.special },
                })
              }
              label={
                <Typography
                  variant="small"
                  className="flex items-center font-normal text-foreground"
                >
                  Special
                </Typography>
              }
              containerProps={{ className: "-ml-2.5" }}
            />
          </div>
          <div className="flex w-full justify-between">
            <Button
              className="mt-6 bg-primary text-foreground"
              onClick={perform_generate}
            >
              Generate
            </Button>
            <Button
              variant="outlined"
              className="mt-6 border-primary text-foreground"
              onClick={() => {
                navigator.clipboard.writeText(password);
              }}
            >
              Copy
            </Button>
          </div>
        </form>
      </div>
      <hr className="w-full border-secondary/40" />
      {password && (
        <div className="mt-6 flex flex-col items-center rounded-lg">
          <div className="app-none-scrollbar flex w-4/5 items-center justify-center overflow-x-scroll text-pretty">
            <Typography className="text-secondary">{password}</Typography>
          </div>
        </div>
      )}
    </div>
  );
}
