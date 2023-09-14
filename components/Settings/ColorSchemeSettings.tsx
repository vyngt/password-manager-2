"use client";
import { Typography } from "@material-tailwind/react";
import { ColorSchemeManager } from "../ColorSchemeManager";

export default function ColorSchemeSettings() {
  return (
    <div className="mb-5 h-full w-full">
      <Typography variant="h6" className="border-b border-pm-secondary pl-1">
        Color Schemes
      </Typography>
      <div className="flex w-full gap-3 pt-4">
        <ColorSchemeManager />
      </div>
    </div>
  );
}
