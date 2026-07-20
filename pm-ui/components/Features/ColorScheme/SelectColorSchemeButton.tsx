"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { IconButton } from "../../MaterialTailwind";
import { faWandMagicSparkles } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/tauri";
import { useThemeDispatch } from "@/components/Features/Theme/hooks";

export function SelectColorSchemeButton({ id }: { id: number }) {
  const dispatch = useThemeDispatch();

  const performSelect = async () => {
    const result = await invoke<boolean>("save_theme_cs", { id });
    if (result) dispatch({ type: "color_scheme/set", payload: id });
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
