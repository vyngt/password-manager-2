export interface BaseColorScheme {
  primary: string;
  secondary: string;
  success: string;
  danger: string;
  warning: string;
  foreground: string;
  background: string;
  selection: string;
}

export interface IBaseColorScheme extends BaseColorScheme {
  name: string;
}
export interface IColorScheme extends IBaseColorScheme {
  id: number;
}

export type IBaseManager<T> = {
  [P in keyof T]: (value: T[P]) => void;
};

// not need?...
export type CSSColorSchemeProperties = {
  [P in keyof BaseColorScheme as `--${P}`]: BaseColorScheme[P];
};

export type IThemeContextProps = {
  color_scheme: IColorScheme;
  initialize: () => void;
  change_to: (id: number) => void;
  set_color_scheme: (scheme: IColorScheme) => void;
};
