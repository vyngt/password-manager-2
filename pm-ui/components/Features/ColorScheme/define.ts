import type { DataModel } from "../../UI/TreeView/define";

interface ColorSchemeBase {
  primary: string;
  secondary: string;
  success: string;
  warning: string;
  danger: string;
  foreground: string;
  background: string;
}

interface ColorScheme extends DataModel, ColorSchemeBase {
  id: number;
  name: string;
}

export type { ColorScheme };
