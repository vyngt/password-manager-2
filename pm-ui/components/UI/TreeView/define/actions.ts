import type { DataModel, TreeData } from "./models";

export interface LoadDataAction<T extends DataModel> {
  type: "load";
  payload: {
    data: TreeData<T>;
  };
}

export interface FilterDataAction<T extends DataModel> {
  type: "filter";
  payload: {
    data: TreeData<T>;
    searchTerm: string;
    page?: number;
  };
}

export interface PaginateDataAction<T extends DataModel> {
  type: "paginate";
  payload: {
    data: TreeData<T>;
    nextPage: number;
  };
}

export type TreeAction<T extends DataModel> =
  | LoadDataAction<T>
  | FilterDataAction<T>
  | PaginateDataAction<T>;
