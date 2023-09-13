export interface BaseColorScheme {
  primary: string;
  secondary: string;
  success: string;
  danger: string;
  warning: string;
  foreground: string;
  background: string;
}
export interface IColorScheme extends BaseColorScheme {
  id: number;
  name: string;
}

// not need?...
export type CSSColorSchemeProperties = {
  [P in keyof BaseColorScheme as `--${P}`]: BaseColorScheme[P];
};

export type IThemeContextProps = {
  color_scheme: IColorScheme;
  change_to: (id: number) => void;
  set_color_scheme: (scheme: IColorScheme) => void;
};
