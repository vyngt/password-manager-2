import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faShieldHalved,
  faGear,
  faWandMagicSparkles,
  faUnlockKeyhole,
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
    icon: faShieldHalved,
    href: "/main/vault",
    sequence: 1,
  },
  {
    type: "action",
    id: "password_generator",
    name: "Password Generator",
    icon: faUnlockKeyhole,
    href: "/main/password-generator",
    sequence: 2,
  },
  {
    type: "action",
    id: "theme",
    name: "Theme",
    icon: faWandMagicSparkles,
    href: "/main/theme",
    sequence: 3,
  },
  {
    type: "action",
    id: "setting",
    name: "Settings",
    icon: faGear,
    href: "/main/settings",
    sequence: 11,
  },
];
