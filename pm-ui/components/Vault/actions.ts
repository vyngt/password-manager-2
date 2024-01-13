import type { ResultWithCount, Item } from "./define";

interface LoadItemsAction {
  type: "load";
  payload: {
    data: ResultWithCount<Item>;
  };
}

interface PerformFilterAction {
  type: "filter";
  payload: {
    data: ResultWithCount<Item>;
    searchTerm: string;
    page?: number;
  };
}

interface PerformPaginateAction {
  type: "paginate";
  payload: {
    data: ResultWithCount<Item>;
    nextPage: number;
  };
}

export type IVaultAction =
  | LoadItemsAction
  | PerformFilterAction
  | PerformPaginateAction;
