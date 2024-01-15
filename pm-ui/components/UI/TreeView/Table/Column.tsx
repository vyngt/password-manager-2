"use client";

import { Typography } from "@/components/MaterialTailwind";

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

export function Column({ children }: { children: React.ReactNode }) {
  return <td className="py-2">{children}</td>;
}
