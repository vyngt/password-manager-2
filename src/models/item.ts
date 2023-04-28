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

interface NewItem extends BaseItem {
  password: string;
}

interface FormItem extends NewItem {
  id?: number;
}

export type { GItem, Item, NewItem, BaseItem, FormItem };

export interface Operator<T extends BaseItem> {
  copy: (item: T) => void;
  delete: (item: T) => void;
  update: (item: T) => void;
}

export interface ItemInputUpdater<T extends ChangeEvent<HTMLInputElement>> {
  update_name: (e: T) => void;
  update_url: (e: T) => void;
  update_username: (e: T) => void;
  update_password: (e: T) => void;
}
