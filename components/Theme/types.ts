export interface IColorScheme {
  id: number;
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
};
