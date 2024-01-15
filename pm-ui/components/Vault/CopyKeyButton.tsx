"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { IconButton } from "../MaterialTailwind";
import { faKey } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/tauri";

export function CopyKeyButton({ id }: { id: number }) {
  const performCopyKeyToClipboard = async () => {
    const result = await invoke<string>("get_item_key", { id });
    navigator.clipboard.writeText(result);
  };

  return (
    <IconButton
      onClick={performCopyKeyToClipboard}
      variant="text"
      className="text-secondary hover:bg-primary/20 active:bg-primary/40"
    >
      <FontAwesomeIcon icon={faKey} />
    </IconButton>
  );
}
