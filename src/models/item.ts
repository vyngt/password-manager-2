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

export type { GItem, Item, NewItem };

export interface Operator {
  copy: (item: GItem) => void;
  delete: (item: GItem) => void;
  update: (item: GItem) => void;
}
