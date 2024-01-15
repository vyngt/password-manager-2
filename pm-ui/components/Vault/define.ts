import type { DataModel } from "../UI/TreeView/define";

interface ItemBase {
  name: string;
  url: string;
  username: string;
}

interface ItemForm extends ItemBase {
  password: string;
}

interface Item extends DataModel, ItemBase {
  id: number;
}

interface ItemInDB extends ItemBase {
  id: number;
  password: string;
}

export type { Item, ItemForm, ItemInDB };
