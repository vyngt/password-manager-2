"use client";

import { Typography } from "@material-tailwind/react";
import { MasterPassword } from "@/components/Settings";

export default function Page() {
  return (
    <div>
      <Typography variant="h3" className="text-center">
        Settings
      </Typography>
      <MasterPassword />
    </div>
  );
}
