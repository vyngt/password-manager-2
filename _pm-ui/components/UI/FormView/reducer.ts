import { FormModel } from "./models";

type FieldValuePayload<T extends FormModel> = {
  [P in keyof T]: {
    field: P;
    value: T[P];
  };
}[keyof T];

export interface PatchFormAction<T extends FormModel> {
  type: "patch";
  payload: FieldValuePayload<T>;
}

export interface SetIDAction<T extends FormModel> {
  type: "set/id";
  payload: number;
}

export interface PutFormAction<T extends FormModel> {
  type: "put";
  payload: T;
}

export type FormAction<T extends FormModel> =
  | PatchFormAction<T>
  | PutFormAction<T>
  | SetIDAction<T>;

export function createFormReducer<T extends FormModel>() {
  const reducer = (state: T, action: FormAction<T>): T => {
    switch (action.type) {
      case "put":
        return { ...state, ...action.payload };
      case "set/id":
        return { ...state, id: action.payload };
      case "patch":
        return { ...state, [action.payload.field]: action.payload.value };
    }
  };

  return reducer;
}
