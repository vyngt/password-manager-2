"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Tooltip } from "@/components/MaterialTailwind";
import type { ISidebarAction } from "./items";
import "./item.css";

export const Item = ({ item }: { item: ISidebarAction }) => {
  const pathname = usePathname();

  const is_selected = pathname == item.href;

  return (
    <Tooltip
      className="border border-secondary bg-background text-foreground"
      content={item.name}
      placement="right"
    >
      <Link
        className={`w-full p-5 ${
          is_selected ? "panel--item-selected" : "panel--item"
        }`}
        href={item.href}
      >
        <div className="flex justify-center">
          <FontAwesomeIcon className="h-full w-full grow" icon={item.icon} />
        </div>
      </Link>
    </Tooltip>
  );
};
