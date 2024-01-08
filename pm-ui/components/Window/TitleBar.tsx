"use client";

import {
  faX,
  faWindowMinimize,
  faWindowMaximize,
} from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { IconButton, Typography } from "../MaterialTailwind";
import { useCallback } from "react";
import Image from "next/image";
import favicon from "./favicon.ico";

export function WindowTitleBar() {
  const getAppWindow = useCallback(async () => {
    if (typeof window == "undefined") return;
    return (await import("@tauri-apps/api/window")).appWindow;
  }, []);

  return (
    <header
      data-tauri-drag-region
      className="flex h-10 flex-grow-0 justify-between border-b border-secondary"
    >
      <div className="pointer-events-none flex flex-col justify-center pl-2">
        <div className="flex gap-2">
          <div className="flex flex-col justify-center">
            <Image src={favicon} width={24} height={24} alt="" />
          </div>
          <Typography variant="h5">Password Manager</Typography>
        </div>
      </div>
      <div
        className="pointer-events-none flex-grow"
        draggable
        onDragStart={async () => {
          const appWindow = await getAppWindow();
          if (appWindow) await appWindow.startDragging();
        }}
      ></div>
      <div className="flex h-full">
        <IconButton
          className="rounded-none text-foreground hover:bg-primary/20 active:bg-primary/40"
          variant="text"
          onClick={async () => {
            const appWindow = await getAppWindow();
            if (appWindow) await appWindow.minimize();
          }}
        >
          <FontAwesomeIcon icon={faWindowMinimize} />
        </IconButton>
        <IconButton
          className="rounded-none text-foreground hover:bg-primary/20 active:bg-primary/40"
          variant="text"
          onClick={async () => {
            const appWindow = await getAppWindow();
            if (!appWindow) return;
            if (await appWindow.isMaximized()) {
              await appWindow.unmaximize();
            } else {
              await appWindow.maximize();
            }
          }}
        >
          <FontAwesomeIcon icon={faWindowMaximize} />
        </IconButton>
        <IconButton
          className="rounded-none text-foreground hover:bg-danger/40 active:bg-danger/60"
          variant="text"
          onClick={async () => {
            const appWindow = await getAppWindow();
            if (appWindow) await appWindow.close();
          }}
        >
          <FontAwesomeIcon icon={faX} />
        </IconButton>
      </div>
    </header>
  );
}
