import type { ThemeAction, ThemeState } from "./define";

export function ThemeReducer(
  state: ThemeState,
  action: ThemeAction,
): ThemeState {
  switch (action.type) {
    case "color_scheme/set":
      return { ...state, colorSchemeId: action.payload };
  }
}
