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
