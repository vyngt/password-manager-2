"use client";

import { Typography } from "@material-tailwind/react";
import { MasterPassword, ColorSchemeSettings } from "@/components/Settings";

export default function Page() {
  return (
    <div className="h-full w-full">
      <Typography variant="h3" className="pt-2 text-center text-pm-primary">
        Settings
      </Typography>
      <MasterPassword />
      <ColorSchemeSettings />
    </div>
  );
}
