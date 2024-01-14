export interface Data {
  id: number;
}

export interface TreeData<T extends Data> {
  result: Array<T>;
  total: number;
}
