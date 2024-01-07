"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

import type { ISidebarAction } from "./items";
import "./item.css";

export const Item = ({ item }: { item: ISidebarAction }) => {
  const pathname = usePathname();

  const is_selected = pathname == item.href;

  return (
    <Link
      className={`flex gap-2 p-3 ${
        is_selected ? "panel-item-selected" : "panel-item"
      }`}
      href={item.href}
    >
      <div className="flex flex-col justify-center">
        <FontAwesomeIcon icon={item.icon} />
      </div>
      <div>{item.name}</div>
    </Link>
  );
};
