"use client";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faChevronLeft,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import { IconButton, Typography } from "@/components/MaterialTailwind";

import { useState, useEffect } from "react";
import { LIMIT, ResultWithCount, Item } from "./define";
import { invoke } from "@tauri-apps/api/tauri";
import { useVault } from "./hooks";

export function VaultPagination() {
  const { state, dispatch } = useVault();
  const [totalPage, setTotalPage] = useState(1);

  const performGetData = async (page: number) => {
    const data = await invoke<ResultWithCount<Item>>("fetch_items", {
      page: page,
      term: state.lastTerm,
    });

    dispatch({ type: "paginate", payload: { data, nextPage: page } });
  };

  const handleNext = async () => {
    await performGetData(state.currentPage + 1);
  };

  const handlePrevious = async () => {
    await performGetData(state.currentPage - 1);
  };

  useEffect(() => {
    const base = Math.floor(state.itemCount / LIMIT);
    const plus = state.itemCount % LIMIT !== 0;
    setTotalPage(plus ? base + 1 : base);
  }, [state.items]);

  return (
    <div className="flex items-center justify-center gap-2">
      <Typography variant="lead">
        {state.currentPage}/{totalPage}
      </Typography>
      <div className="flex justify-center gap-1">
        <IconButton
          disabled={state.currentPage === 1}
          onClick={handlePrevious}
          variant="text"
          className="bg-secondary/20 text-foreground hover:bg-secondary/30 active:bg-secondary/60"
        >
          <FontAwesomeIcon icon={faChevronLeft} />
        </IconButton>
        <IconButton
          disabled={state.currentPage === totalPage}
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
