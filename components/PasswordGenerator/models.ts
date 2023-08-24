interface IPasswordGenerator {
  len: number;
  upper: boolean;
  lower: boolean;
  digits: boolean;
  special: boolean;
}

export type { IPasswordGenerator };
