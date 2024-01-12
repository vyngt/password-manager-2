export const LIMIT = 40;

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

interface ItemInDB extends ItemBase {
  id: number;
  password: string;
}

// For Pagination
interface ResultWithCount<T> {
  result: Array<T>;
  total: number;
}

export type { Item, ItemCreate, ItemUpdate, ItemInDB, ResultWithCount };
