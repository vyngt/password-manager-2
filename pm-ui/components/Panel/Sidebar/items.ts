import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faBook,
  faGear,
  faWandMagicSparkles,
} from "@fortawesome/free-solid-svg-icons";

export interface ISidebarAction {
  type: "action";
  id: string;
  name: string;
  icon: IconDefinition;
  href: string;
  sequence: number;
}

export type ISidebarItem = ISidebarAction;

export const SidebarManager = {
  all: () => {
    return registry.sort((a, b) => a.sequence - b.sequence);
  },
};

export const registry: ISidebarItem[] = [
  {
    type: "action",
    id: "receipt",
    name: "Receipts",
    icon: faBook,
    href: "/",
    sequence: 1,
  },
  {
    type: "action",
    id: "theme",
    name: "Theme",
    icon: faWandMagicSparkles,
    href: "/theme",
    sequence: 3,
  },
  {
    type: "action",
    id: "setting",
    name: "Settings",
    icon: faGear,
    href: "/settings",
    sequence: 11,
  },
];
