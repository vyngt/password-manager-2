"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faChevronLeft,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import { IconButton, Typography } from "@/components/MaterialTailwind";

export function VaultPagination() {
  return (
    <div className="flex items-center justify-center gap-2">
      <Typography variant="lead">1/2</Typography>
      <div className="flex justify-center gap-1">
        <IconButton
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronLeft} />
        </IconButton>
        <IconButton
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronRight} />
        </IconButton>
      </div>
    </div>
  );
}
