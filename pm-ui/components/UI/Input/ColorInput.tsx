"use client";
import { Typography } from "@/components/MaterialTailwind";
import "./ColorInput.css";

interface ColorInputProps extends Omit<React.ComponentProps<"input">, "size"> {
  name: string;
}

export function ColorInput({ name, ...rest }: ColorInputProps) {
  return (
    <div className="relative flex flex-col items-center justify-center gap-1">
      <div className="flex w-full justify-center">
        <label className="app--ui--color-container">
          <input {...rest} type="color" />
        </label>
      </div>
      <Typography className="capitalize text-primary">{name}</Typography>
    </div>
  );
}
