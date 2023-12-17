"use client";
import "./style.css";
import { ColorSchemeManagerHeader } from "./ColorSchemeHeader";

import { ColorSchemeManagerProvider } from "./Context";

import ColorSchemeTable from "./ColorSchemeTable";

export default function ColorSchemeManager() {
  return (
    <ColorSchemeManagerProvider>
      <div className="flex h-full w-full flex-col">
        <div className="p-3">
          <ColorSchemeManagerHeader />
        </div>
        <ColorSchemeTable />
      </div>
    </ColorSchemeManagerProvider>
  );
}
