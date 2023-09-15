import { createContext, useState, useContext } from "react";
import { IColorScheme } from "../Theme/types";
import { invoke } from "@tauri-apps/api/tauri";

interface IColorSchemeManagerContext {
  schemes: Array<IColorScheme>;
  reload: () => void;
  add: (scheme: IColorScheme) => void;
  remove: (scheme_id: number) => void;
  update: (scheme: IColorScheme) => void;
}

export const ColorSchemeManagerContext =
  createContext<IColorSchemeManagerContext>({
    schemes: [],
    reload: () => {},
    add: () => {},
    remove: () => {},
    update: () => {},
  });

export const ColorSchemeManagerProvider = ({
  children,
}: {
  children: React.ReactNode;
}) => {
  const [schemes, setSchemes] = useState<Array<IColorScheme>>([]);

  const context: IColorSchemeManagerContext = {
    schemes,
    reload: async () => {
      const records: Array<IColorScheme> = await invoke(
        "get_all_color_schemes",
      );
      setSchemes(records);
    },
    add: async (scheme: IColorScheme) => {
      setSchemes((cur) => [...cur, scheme]);
    },
    remove: async (scheme_id: number) => {
      let new_arr = [...schemes];
      const deleted = await invoke("delete_color_scheme", { id: scheme_id });
      if (!deleted) {
        return;
      }
      const index = schemes.findIndex((i) => i.id == scheme_id);
      new_arr.splice(index, 1);
      setSchemes(new_arr);
    },
    update: (scheme: IColorScheme) => {},
  };

  return (
    <ColorSchemeManagerContext.Provider value={context}>
      {children}
    </ColorSchemeManagerContext.Provider>
  );
};

export const useManager = () => {
  const context = useContext(ColorSchemeManagerContext);
  return context;
};
