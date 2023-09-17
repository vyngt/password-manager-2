import { createContext, useState, useContext } from "react";
import { IBaseColorScheme, IColorScheme } from "../Theme/types";
import { invoke } from "@tauri-apps/api/tauri";

interface IColorSchemeManagerContext {
  schemes: Array<IColorScheme>;
  reload: () => void;
  add: (scheme: IBaseColorScheme) => void;
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
    add: async (scheme: IBaseColorScheme) => {
      const result: IColorScheme = await invoke("create_color_scheme", {
        data: scheme,
      });
      setSchemes((cur) => [...cur, result]);
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
    update: async (scheme: IColorScheme) => {
      const updated = await invoke("update_color_scheme", { data: scheme });
      if (!updated) {
        return;
      }
      const new_arr = [...schemes];
      const index = new_arr.findIndex((i) => i.id == scheme.id);
      new_arr[index] = scheme;
      setSchemes(new_arr);
    },
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
