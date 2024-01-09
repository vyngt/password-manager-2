interface PassConfLength {
  len: number;
}

interface PassConfOther {
  upper: boolean;
  lower: boolean;
  digits: boolean;
  special: boolean;
}

type OfType<T> = {
  [K in keyof T]: {
    key: K;
    value: T[K];
  };
}[keyof T];

export interface PasswordConfiguration extends PassConfLength, PassConfOther {}

interface ConfigureLength {
  type: "len";
  payload: number;
}

interface ConfigureOther {
  type: "other";
  payload: OfType<PassConfOther>;
}

type PGAction = ConfigureLength | ConfigureOther;

export function PGReducer(state: PasswordConfiguration, action: PGAction) {
  switch (action.type) {
    case "len":
      return {
        ...state,
        len: action.payload,
      };
    case "other":
      const new_state = { ...state };
      new_state[action.payload.key] = action.payload.value;
      return new_state;
  }
}
