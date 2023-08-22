import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faVault,
  faDoorClosed,
  faGear,
  faTags,
} from "@fortawesome/free-solid-svg-icons";

export interface ISidebarItem {
  id: string;
  name: string;
  icon: IconDefinition;
  href: string;
  sequence: number;
}

interface ISidebarRegistry {
  [key: string]: ISidebarItem;
}

export const SidebarManager = {
  all: () => {
    return registry.sort((a, b) => a.sequence - b.sequence);
  },
};

export const registry: ISidebarItem[] = [
  {
    id: "vault",
    name: "Vault",
    icon: faVault,
    href: "/main/vault",
    sequence: 1,
  },
  {
    id: "exit",
    name: "Exit",
    icon: faDoorClosed,
    href: "/exit",
    sequence: 99,
  },
  {
    id: "tags",
    name: "Tags",
    icon: faTags,
    href: "/main/tags",
    sequence: 3,
  },
  {
    id: "settings",
    name: "Settings",
    icon: faGear,
    href: "/main/settings",
    sequence: 4,
  },
];
