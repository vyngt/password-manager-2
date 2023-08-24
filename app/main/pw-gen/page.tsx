"use client";

import { invoke } from "@tauri-apps/api";
import { FormEvent, useState } from "react";
import {
  Checkbox,
  Card,
  CardHeader,
  CardFooter,
  CardBody,
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
    <Card color="transparent" shadow={false} className="h-full w-full">
      <CardHeader floated={false} shadow={false} className="rounded-none">
        <div className="mb-8 flex flex-col items-center justify-between gap-8">
          <Typography variant="h4" color="blue-gray">
            Password Generator
          </Typography>
          <Typography color="gray" className="mt-1 font-normal">
            Let generate your strong password
          </Typography>
        </div>
      </CardHeader>
      <CardBody className="flex flex-col justify-center items-center">
        <form className="mt-8 mb-2 w-80 max-w-screen-lg sm:w-96 flex flex-col items-center gap-2">
          <Input
            label="Length"
            crossOrigin={""}
            value={state.len}
            type="number"
            onInput={UpdateState.len}
          />
          <div className="flex gap-4">
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
          </div>
          <div className="w-full flex justify-between">
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
      </CardBody>
      <CardFooter className="flex flex-col text-center items-center bg-gray-100">
        <div className="overflow-y">
          <Typography>{password}</Typography>
        </div>
      </CardFooter>
    </Card>
  );
}
