import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { faVault, faDoorClosed } from "@fortawesome/free-solid-svg-icons";

export interface ISidebarItem {
  id: string;
  name: string;
  icon: IconDefinition;
  href: string;
  sequence: number;
}

export const SidebarItems: ISidebarItem[] = [
  { id: "vault", name: "Vault", icon: faVault, href: "/main", sequence: 1 },
  {
    id: "exit",
    name: "Exit",
    icon: faDoorClosed,
    href: "/exit",
    sequence: 1,
  },
];
