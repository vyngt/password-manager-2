import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faGear,
  faPalette,
  faUnlockKeyhole,
  faDownload,
  faVault,
} from "@fortawesome/free-solid-svg-icons";

import type { SidebarItem } from "@/components/UI/Sidebar/define";

interface Item extends SidebarItem {
  name: string;
  icon: IconDefinition;
  href: string;
}

export const items: Array<Item> = [
  {
    id: "vault",
    name: "Vault",
    icon: faVault,
    href: "/main/vault",
    sequence: 1,
  },
  {
    id: "backup",
    name: "Backup",
    icon: faDownload,
    href: "/main/backup",
    sequence: 2,
  },
  {
    id: "password_generator",
    name: "Password Generator",
    icon: faUnlockKeyhole,
    href: "/main/password-generator",
    sequence: 3,
  },
  {
    id: "color-scheme",
    name: "Color Scheme",
    icon: faPalette,
    href: "/main/color-scheme",
    sequence: 4,
  },
  {
    id: "manage",
    name: "Manage",
    icon: faGear,
    href: "/main/manage",
    sequence: 11,
  },
];
