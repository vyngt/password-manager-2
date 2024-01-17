"use client";

import { IconButton } from "@/components/MaterialTailwind";
import { faTrashCan } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { memo } from "react";

export const DeleteAction = memo(function DeleteAction({
  handle,
}: {
  handle: () => void;
}) {
  return (
    <IconButton
      onClick={handle}
      variant="outlined"
      className="border-danger text-danger focus:ring-danger/30"
    >
      <FontAwesomeIcon icon={faTrashCan} />
    </IconButton>
  );
});
