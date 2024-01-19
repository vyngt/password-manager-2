"use client";

import { invoke } from "@tauri-apps/api/tauri";
import { Input } from "@/components/UI";
import { Button, Typography } from "@/components/MaterialTailwind";
import { Section } from "./Section";

import { useReducer, useState, useEffect } from "react";

interface Password {
  password: string;
  password2: string;
}

type FieldValuePayload<T> = {
  [P in keyof T]: {
    field: P;
    value: T[P];
  };
}[keyof T];

interface PatchPasswordAction {
  type: "patch";
  payload: FieldValuePayload<Password>;
}

interface PutPasswordAction {
  type: "put";
  payload: Password;
}

type PasswordAction = PatchPasswordAction | PutPasswordAction;

function reducer(state: Password, action: PasswordAction): Password {
  switch (action.type) {
    case "patch":
      return { ...state, [action.payload.field]: action.payload.value };
    case "put":
      return { ...state, ...action.payload };
  }
}

export function ChangePassword() {
  const [state, dispatch] = useReducer(reducer, {
    password: "",
    password2: "",
  });
  const [check, setCheck] = useState<boolean | null>(null);

  const performChangePassword = async () => {
    const { password, password2 } = state;
    if (password.length == 0) return;
    else if (password !== password2) {
      setCheck(false);
      return;
    } else {
      const ok = await invoke<boolean>("rekey_auth", { password });
      setCheck(ok);
      dispatch({ type: "put", payload: { password: "", password2: "" } });
    }
  };

  useEffect(() => {
    if (check !== null) {
      const t = setTimeout(() => {
        setCheck(null);
      }, 3000);
      return () => clearTimeout(t);
    }
  }, [check]);

  return (
    <Section name="Change Master Password">
      <div className="flex max-w-[400px] flex-col gap-3">
        <Input
          type="password"
          color="primary"
          label="New Password"
          value={state.password}
          onChange={(ev) =>
            dispatch({
              type: "patch",
              payload: { field: "password", value: ev.target.value },
            })
          }
        />
        <Input
          type="password"
          color="primary"
          label="Confirm Password"
          value={state.password2}
          onChange={(ev) =>
            dispatch({
              type: "patch",
              payload: { field: "password2", value: ev.target.value },
            })
          }
        />
        <div>
          <Button
            onClick={performChangePassword}
            disabled={check !== null}
            className="bg-primary/50 text-foreground"
          >
            Update
          </Button>
        </div>
        {check === null ? (
          ""
        ) : check === true ? (
          ""
        ) : (
          <Typography className="font-bold text-warning">
            Password not match!
          </Typography>
        )}
      </div>
    </Section>
  );
}
