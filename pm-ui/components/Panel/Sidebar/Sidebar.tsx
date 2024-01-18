"use client";

import Link from "next/link";
import { items } from "./items";
import { SidebarWrapper } from "@/components/UI/Sidebar";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Tooltip } from "@/components/MaterialTailwind";

import "./Sidebar.css";

export const Wrapper = SidebarWrapper(items);
export const Sidebar = Wrapper(({ item }) => {
  const pathname = usePathname();
  const selected = () => {
    const reg = new RegExp(`^${item.href}.*`);
    return reg.test(pathname);
  };

  return (
    <Tooltip
      className="panel--item-tooltip-arrow panel--item-tooltip panel--item-tooltip-transform bg-background text-foreground"
      content={<div className="relative z-50">{item.name}</div>}
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
});
