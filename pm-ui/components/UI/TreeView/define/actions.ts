import type { DataModel, TreeData } from "./models";

interface LoadDataAction<T extends DataModel> {
  type: "load";
  payload: {
    data: TreeData<T>;
  };
}

interface FilterDataAction<T extends DataModel> {
  type: "filter";
  payload: {
    data: TreeData<T>;
    searchTerm: string;
    page?: number;
  };
}

interface PaginateDataAction<T extends DataModel> {
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
