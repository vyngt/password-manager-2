"use client";

import { invoke } from "@tauri-apps/api";
import { FormEvent, useState } from "react";
import {
  Checkbox,
  Card,
  Typography,
  Button,
  Textarea,
  Input,
} from "@material-tailwind/react";

interface IPasswordGenerator {
  len: number;
  upper: boolean;
  lower: boolean;
  digits: boolean;
  special: boolean;
}

export default function Page() {
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
      setState({ ...state, len: Number(ev.currentTarget.value) });
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
    <Card color="transparent" shadow={false}>
      <Typography variant="h4" color="blue-gray">
        Password Generator
      </Typography>
      <Typography color="gray" className="mt-1 font-normal">
        Let generate your strong password
      </Typography>
      <form className="mt-8 mb-2 w-80 max-w-screen-lg sm:w-96">
        <Input
          label="Length"
          crossOrigin={""}
          value={state.len}
          type="number"
          onInput={UpdateState.len}
        />
        <Checkbox
          crossOrigin={""}
          checked={state.lower}
          onChange={UpdateState.lower}
          label={
            <Typography
              variant="small"
              color="gray"
              className="flex items-center font-normal"
            >
              Lowercase
            </Typography>
          }
          containerProps={{ className: "-ml-2.5" }}
        />
        <Checkbox
          crossOrigin={""}
          checked={state.upper}
          onChange={UpdateState.upper}
          label={
            <Typography
              variant="small"
              color="gray"
              className="flex items-center font-normal"
            >
              Uppercase
            </Typography>
          }
          containerProps={{ className: "-ml-2.5" }}
        />
        <Checkbox
          crossOrigin={""}
          checked={state.digits}
          onChange={UpdateState.digits}
          label={
            <Typography
              variant="small"
              color="gray"
              className="flex items-center font-normal"
            >
              Digits
            </Typography>
          }
          containerProps={{ className: "-ml-2.5" }}
        />
        <Checkbox
          crossOrigin={""}
          checked={state.special}
          onChange={UpdateState.special}
          label={
            <Typography
              variant="small"
              color="gray"
              className="flex items-center font-normal"
            >
              Special
            </Typography>
          }
          containerProps={{ className: "-ml-2.5" }}
        />
        <div className="w-96">
          <Textarea label="Password" value={password} onChange={() => {}} />
        </div>
        <div className="flex">
          <Button className="mt-6" onClick={perform_generate}>
            Generate
          </Button>
          <Button
            className="mt-6"
            onClick={() => {
              navigator.clipboard.writeText(password);
            }}
          >
            Copy
          </Button>
        </div>
      </form>
    </Card>
  );
}
