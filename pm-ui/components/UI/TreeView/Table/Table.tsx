"use client";

import { Typography } from "@/components/MaterialTailwind";

export function Table({
  children,
  headers,
}: {
  children: React.ReactNode;
  headers: Array<string>;
}) {
  return (
    <div className="app-scrollbar grow overflow-y-scroll">
      <table className="w-full min-w-max table-auto text-left">
        <thead>
          <tr className="border-b border-b-secondary/75 bg-primary/10 text-primary">
            {headers.map((head) => (
              <th key={head} className="p-4 capitalize">
                <Typography
                  variant="small"
                  className="flex items-center justify-between gap-2 font-bold leading-none"
                >
                  {head}
                </Typography>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}
