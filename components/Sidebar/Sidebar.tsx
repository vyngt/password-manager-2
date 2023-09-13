"use client";

import { List, ListItem, ListItemPrefix } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { ISidebarItem, SidebarManager } from "./items";

import { useRouter, usePathname } from "next/navigation";
import { useColorScheme } from "../Theme";

const SidebarItem = ({ item }: { item: ISidebarItem }) => {
  const router = useRouter();
  const pathname = usePathname();
  const { primary } = useColorScheme();

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
