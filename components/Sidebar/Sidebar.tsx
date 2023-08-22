"use client";

import { List, ListItem, ListItemPrefix } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { ISidebarItem, SidebarManager } from "./items";

import { useRouter, usePathname } from "next/navigation";

const SidebarItem = ({ item }: { item: ISidebarItem }) => {
  const router = useRouter();
  const pathname = usePathname();

  return (
    <ListItem
      className={`rounded-none ${pathname == item.href ? "text-white" : ""}`}
      onClick={() => {
        router.push(item.href);
      }}
    >
      <ListItemPrefix>
        <FontAwesomeIcon icon={item.icon} className="h-5 w-5" />
      </ListItemPrefix>
      {item.name}
    </ListItem>
  );
};

export default function Sidebar() {
  return (
    <div className="h-full w-full max-w-[20rem] shadow-xl bg-black">
      <List>
        {SidebarManager.all().map((e) => (
          <SidebarItem key={e.id} item={e} />
        ))}
      </List>
    </div>
  );
}
