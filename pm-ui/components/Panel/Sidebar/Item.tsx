"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Tooltip } from "@/components/MaterialTailwind";
import type { ISidebarAction } from "./items";
import "./item.css";

export const Item = ({ item }: { item: ISidebarAction }) => {
  const pathname = usePathname();
  const selected = () => {
    const reg = new RegExp(`^${item.href}.*`);
    return reg.test(pathname);
  };

  return (
    <Tooltip
      className="border border-secondary bg-background text-foreground"
      content={item.name}
      placement="right"
    >
      <Link
        className={`w-full p-5 ${
          selected() ? "panel--item-selected" : "panel--item"
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
