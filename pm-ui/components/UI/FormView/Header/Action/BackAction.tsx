"use client";
import { IconButton } from "@/components/MaterialTailwind";
import { faArrowLeft } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useCallback } from "react";
import { useRouter } from "next/navigation";

export function BackAction({ href }: { href?: string }) {
  const router = useRouter();

  const handle = useCallback(() => {
    href ? router.push(href) : router.back();
  }, [router, href]);

  return (
    <IconButton
      onClick={handle}
      variant="filled"
      className="bg-primary/70 text-foreground"
    >
      <FontAwesomeIcon icon={faArrowLeft} />
    </IconButton>
  );
}
