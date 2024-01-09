// Model: Item
interface ItemBase {
  name: string;
  url: string;
  username: string;
}

interface ItemCreate extends ItemBase {
  password: string;
}

interface ItemUpdate extends ItemCreate {
  id: number;
}

interface Item extends ItemBase {
  id: number;
}

export type { Item, ItemCreate, ItemUpdate };
