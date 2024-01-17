"use client";

import { Typography } from "@/components/MaterialTailwind";
import classnames from "classnames";

export function FieldColumn({
  data,
  onClick,
}: {
  data: React.ReactNode;
  onClick?: () => void;
}) {
  return (
    <td className={`py-2${onClick ? " cursor-pointer" : ""}`} onClick={onClick}>
      <div className="flex items-center gap-3">
        <div className="flex flex-col">
          <Typography variant="small" className="pl-4 font-normal">
            {data}
          </Typography>
        </div>
      </div>
    </td>
  );
}

interface ColumnProps extends Omit<React.ComponentProps<"td">, "size"> {
  children: React.ReactNode;
}

export function Column({ children, className, ...rest }: ColumnProps) {
  const c = classnames("py-2", className);
  return (
    <td {...rest} className={c}>
      {children}
    </td>
  );
}
