"use client";

import { List, ListItem, ListItemPrefix } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { ISidebarItem, SidebarManager } from "./items";
import Link from "next/link";

import { usePathname } from "next/navigation";

const SidebarItem = ({ item }: { item: ISidebarItem }) => {
  const pathname = usePathname();

  return (
    <Link
      className={`flex gap-2 p-3 ${
        pathname == item.href
          ? "text-pm-primary pointer-events-none"
          : "text-pm-secondary hover:text-pm-foreground transition-colors"
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

const Sidebar: React.FunctionComponent<
  React.HTMLAttributes<HTMLDivElement>
> = ({ ...rest }) => {
  return (
    <div {...rest}>
      <List>
        {SidebarManager.all().map((e) => (
          <SidebarItem key={e.id} item={e} />
        ))}
      </List>
    </div>
  );
};

export default Sidebar;
