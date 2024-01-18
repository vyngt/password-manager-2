"use client";

import classnames from "classnames";
import Link from "next/link";

import { Typography } from "@/components/MaterialTailwind";
import { SidebarWrapper } from "@/components/UI/Sidebar";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { items } from "./items";

import "./Sidebar.css";

export const Wrapper = SidebarWrapper(items);
export const Sidebar = Wrapper(({ item }) => {
  const pathname = usePathname();
  const selected = () => {
    const reg = new RegExp(`^${item.href}.*`);
    return reg.test(pathname);
  };

  const classes = classnames(
    "w-full p-3 manage--item",
    selected() ? "selected" : "",
  );

  return (
    <Link className={classes} href={item.href}>
      <div className="flex gap-2">
        <div className="flex flex-col justify-center">
          <FontAwesomeIcon icon={item.icon} />
        </div>
        <Typography className="capitalize text-foreground">
          {item.name}
        </Typography>
      </div>
    </Link>
  );
});
