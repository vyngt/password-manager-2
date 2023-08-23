import { ChangeEvent } from "react";

interface BaseItem {
  name: string;
  url: string;
  username: string;
}

interface GItem extends BaseItem {
  id: number;
}

interface Item extends GItem {
  password: string;
}

/**
 * For Item
 */
interface ItemManager {
  name: (name: string) => void;
  url: (url: string) => void;
  username: (username: string) => void;
  password: (password: string) => void;
  clear: () => void;
}

export type { GItem, Item, BaseItem, ItemManager };

export interface Operator<T extends BaseItem> {
  copy: (item: T) => void;
  delete: (item: T) => void;
  update: (item: T) => void;
}

export interface IVaultHeaderManager {
  refresh: () => void;
}
