import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faPalette,
  faUserShield,
  faDownload,
} from "@fortawesome/free-solid-svg-icons";

import type { SidebarItem } from "@/components/UI/Sidebar/define";

interface Item extends SidebarItem {
  name: string;
  icon: IconDefinition;
  href: string;
}

export const items: Array<Item> = [
  {
    id: "manage/profile",
    name: "Profile",
    icon: faUserShield,
    href: "/main/manage/profile",
    sequence: 1,
  },
  {
    id: "manage/backup",
    name: "Backup",
    icon: faDownload,
    href: "/main/manage/backup",
    sequence: 2,
  },
  {
    id: "manage/color-scheme",
    name: "Color Scheme",
    icon: faPalette,
    href: "/main/manage/color-scheme",
    sequence: 3,
  },
];
