"use client";

import { IconButton } from "@/components/MaterialTailwind";
import { faFloppyDisk } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

export function SaveAction({ handle }: { handle: () => void }) {
  return (
    <IconButton
      onClick={handle}
      variant="outlined"
      className="border-success text-success focus:ring-success/30"
    >
      <FontAwesomeIcon icon={faFloppyDisk} />
    </IconButton>
  );
}
