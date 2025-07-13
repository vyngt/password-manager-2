export interface DataModel {
  id: number;
}

export interface TreeData<T extends DataModel> {
  result: Array<T>;
  total: number;
}
