"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faChevronLeft,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import { IconButton, Typography } from "@/components/MaterialTailwind";

export function Paginator({
  currentPage,
  totalPage,
  handleNext,
  handlePrevious,
}: {
  currentPage: number;
  totalPage: number;
  handleNext: () => void;
  handlePrevious: () => void;
}) {
  return (
    <div className="flex items-center justify-center gap-2">
      <Typography variant="lead">
        {currentPage}/{totalPage}
      </Typography>
      <div className="flex justify-center gap-1">
        <IconButton
          disabled={currentPage === 1}
          onClick={handlePrevious}
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronLeft} />
        </IconButton>
        <IconButton
          disabled={currentPage === totalPage}
          onClick={handleNext}
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronRight} />
        </IconButton>
      </div>
    </div>
  );
}
