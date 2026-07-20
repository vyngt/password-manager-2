"use client";

import Link from "next/link";
import { IconButton, Button } from "@/components/MaterialTailwind";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate } from "@fortawesome/free-solid-svg-icons";

/**
 * Navigate to form
 */
export function FormAction({ href }: { href: string }) {
  return (
    <Link href={href}>
      <Button variant="filled" className="bg-primary/70 text-foreground">
        New
      </Button>
    </Link>
  );
}

export function RefreshAction({ handle }: { handle: () => void }) {
  return (
    <IconButton
      onClick={handle}
      variant="outlined"
      className="border-secondary text-secondary focus:ring-secondary/30"
    >
      <FontAwesomeIcon icon={faArrowsRotate} />
    </IconButton>
  );
}
