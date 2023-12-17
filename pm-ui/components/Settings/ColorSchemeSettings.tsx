"use client";
import { Typography } from "@material-tailwind/react";
import { ColorSchemeManager } from "../ColorSchemeManager";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPalette } from "@fortawesome/free-solid-svg-icons";

export default function ColorSchemeSettings() {
  return (
    <div className="mb-5 h-full w-full">
      <div className="flex gap-2 border-b-2 border-pm-secondary pl-1">
        <div className="flex flex-col justify-center">
          <FontAwesomeIcon icon={faPalette} />
        </div>
        <Typography variant="h6">Color Schemes</Typography>
      </div>
      <div className="h-[400px] w-full pt-4">
        <ColorSchemeManager />
      </div>
    </div>
  );
}
