import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faGear,
  faWandMagicSparkles,
  faUnlockKeyhole,
  faDownload,
  faVault,
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
    id: "vault",
    name: "Vault",
    icon: faVault,
    href: "/main/vault",
    sequence: 1,
  },
  {
    type: "action",
    id: "backup",
    name: "Backup",
    icon: faDownload,
    href: "/main/backup",
    sequence: 2,
  },
  {
    type: "action",
    id: "password_generator",
    name: "Password Generator",
    icon: faUnlockKeyhole,
    href: "/main/password-generator",
    sequence: 3,
  },
  {
    type: "action",
    id: "color-scheme",
    name: "Color Scheme",
    icon: faWandMagicSparkles,
    href: "/main/color-scheme",
    sequence: 4,
  },
  {
    type: "action",
    id: "manage",
    name: "Manage",
    icon: faGear,
    href: "/main/manage",
    sequence: 11,
  },
];
