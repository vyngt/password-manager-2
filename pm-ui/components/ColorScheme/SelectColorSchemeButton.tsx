"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { IconButton } from "../MaterialTailwind";
import { faWandMagicSparkles } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/tauri";

export function SelectColorSchemeButton({ id }: { id: number }) {
  const performSelect = async () => {
    // const result = await invoke<string>("get_item_key", { id });
    // navigator.clipboard.writeText(result);
  };

  return (
    <IconButton
      onClick={performSelect}
      variant="text"
      className="text-success hover:bg-primary/20 active:bg-primary/40"
    >
      <FontAwesomeIcon icon={faWandMagicSparkles} />
    </IconButton>
  );
}
