"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faChevronLeft,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import { IconButton, Typography } from "@/components/MaterialTailwind";

import { useState, useEffect } from "react";

export function Paginator({
  currentPage,
  itemCount,
  limit = 40,
  handlePage,
}: {
  currentPage: number;
  itemCount: number;
  limit?: number;
  handlePage: (page: number) => void;
}) {
  const [totalPage, setTotalPage] = useState(1);

  useEffect(() => {
    const base = Math.floor(itemCount / limit);
    const plus = itemCount % limit !== 0;
    setTotalPage(plus ? base + 1 : base);
  }, [itemCount, limit]);

  return (
    <div className="flex items-center justify-center gap-2">
      <Typography variant="lead">
        {currentPage}/{totalPage}
      </Typography>
      <div className="flex justify-center gap-1">
        <IconButton
          disabled={currentPage === 1}
          onClick={() => handlePage(currentPage - 1)}
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronLeft} />
        </IconButton>
        <IconButton
          disabled={currentPage === totalPage}
          onClick={() => handlePage(currentPage + 1)}
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronRight} />
        </IconButton>
      </div>
    </div>
  );
}
