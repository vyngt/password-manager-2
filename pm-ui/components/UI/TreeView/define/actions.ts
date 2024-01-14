import type { Data, TreeData } from "./models";

interface LoadDataAction<T extends Data> {
  type: "load";
  payload: {
    data: TreeData<T>;
  };
}

interface FilterDataAction<T extends Data> {
  type: "filter";
  payload: {
    data: TreeData<T>;
    searchTerm: string;
    page?: number;
  };
}

interface PaginateDataAction<T extends Data> {
  type: "paginate";
  payload: {
    data: TreeData<T>;
    nextPage: number;
  };
}

export type TreeAction<T extends Data> =
  | LoadDataAction<T>
  | FilterDataAction<T>
  | PaginateDataAction<T>;
