import type { DataModel } from "../UI/TreeView/define";

interface ColorSchemeBase {
  name: string;
  primary: string;
  secondary: string;
  success: string;
  warning: string;
  danger: string;
  foreground: string;
  background: string;
}

interface ColorSchemeForm extends ColorSchemeBase {}

interface ColorScheme extends DataModel, ColorSchemeBase {
  id: number;
}

export type { ColorScheme, ColorSchemeForm };
