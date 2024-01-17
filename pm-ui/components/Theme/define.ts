export interface ThemeState {
  colorSchemeId: number;
}

interface SetColorSchemeAction {
  type: "color_scheme/set";
  payload: number;
}

export type ThemeAction = SetColorSchemeAction;
