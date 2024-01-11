"use client";

import React from "react";
import classnames from "classnames";

type Color = "primary" | "secondary" | "warning" | "danger" | "success";

interface InputProps extends Omit<React.ComponentProps<"input">, "size"> {
  color: Color;
  label: string;
  containerProps?: React.HTMLAttributes<HTMLDivElement>;
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ color, label, containerProps, ...rest }, ref) => {
    const containerClasses = classnames(
      "relative h-10 w-full min-w-[200px]",
      containerProps?.className,
    );

    const inputClasses = classnames(`app--input-${color} peer`, rest.className);

    return (
      <div {...containerProps} className={containerClasses}>
        <input {...rest} className={inputClasses} placeholder=" " ref={ref} />
        <label
          className={`before:content[' '] after:content[' '] app--label-${color}`}
        >
          {label}
        </label>
      </div>
    );
  },
);

Input.displayName = "PasswordManager.Input";

export { Input };
