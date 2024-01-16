"use client";

import { IconButton } from "@/components/MaterialTailwind";
import { faTrashCan } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

export function DeleteAction({ handle }: { handle: () => void }) {
  return (
    <IconButton
      onClick={handle}
      variant="outlined"
      className="border-secondary text-secondary focus:ring-secondary/30"
    >
      <FontAwesomeIcon icon={faTrashCan} />
    </IconButton>
  );
}
