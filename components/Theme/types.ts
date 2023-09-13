export interface IColorScheme {
  id: number;
  name: string;
  primary: string;
  secondary: string;
  success: string;
  danger: string;
  warning: string;
  foreground: string;
  background: string;
}

export type IThemeContextProps = {
  color_scheme: IColorScheme;
  change_to: (id: number) => void;
  set_color_scheme: (scheme: IColorScheme) => void;
};
