"use client";

import type { Dispatch } from "react";
import type { FormModel } from "../models";
import type { FormAction } from "../reducer";

export type FormComponent<T extends FormModel> = ({
  state,
  dispatch,
}: {
  state: T;
  dispatch: Dispatch<FormAction<T>>;
}) => React.JSX.Element;

export function Form({ children }: { children: React.ReactNode }) {
  return (
    <form className="m-3 flex flex-col gap-4 rounded-md border border-secondary/60 p-5">
      {children}
    </form>
  );
}
