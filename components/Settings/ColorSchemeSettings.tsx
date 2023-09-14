"use client";
import { Typography } from "@material-tailwind/react";
import { ColorSchemeManager } from "../ColorSchemeManager";

export default function ColorSchemeSettings() {
  return (
    <div className="mb-5 h-full w-full">
      <Typography variant="h6" className="border-b-2 border-pm-secondary pl-1">
        Color Schemes
      </Typography>
      <div className="h-[500px] w-full pt-4">
        <ColorSchemeManager />
      </div>
    </div>
  );
}
