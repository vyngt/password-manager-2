"use client";
import { ColorScheme } from "./define";

export function SchemeColumn({ data }: { data: ColorScheme }) {
  return (
    <div className="flex gap-1">
      {Object.entries(data).map(
        (d) =>
          !(d[0] == "name" || d[0] == "id") && (
            <div
              key={`${data.id}-${d[0]}`}
              className="h-3 w-3 rounded-md"
              style={{ backgroundColor: `rgb(${d[1]})` }}
            ></div>
          ),
      )}
    </div>
  );
}
