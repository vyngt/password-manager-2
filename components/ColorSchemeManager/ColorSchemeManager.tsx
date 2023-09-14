"use client";
import "./style.css";
import { invoke } from "@tauri-apps/api/tauri";
import { useState, useEffect } from "react";
import { Button } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate, faPlus } from "@fortawesome/free-solid-svg-icons";

import ColorSchemeList from "./ColorSchemeList";
import {
  ColorSchemeCreateForm,
  ColorSchemeUpdateForm,
} from "./ColorSchemeForm";
import { IColorScheme } from "../Theme/types";

export default function ColorSchemeManager() {
  const [schemes, set_schemes] = useState<Array<IColorScheme>>([]);

  const fetch_schemes = async () => {
    const results: Array<IColorScheme> = await invoke("get_all_color_schemes");
    set_schemes(results);
  };

  useEffect(() => {
    const inline_effect = async () => {
      const results: Array<IColorScheme> = await invoke(
        "get_all_color_schemes",
      );
      set_schemes(results);
    };

    inline_effect();
  });

  return (
    <div className="flex h-full w-full flex-col">
      <div className="p-3">
        <div className="flex gap-3">
          <Button
            size="sm"
            onClick={() => {}}
            className="bg-pm-primary text-pm-foreground"
          >
            <FontAwesomeIcon icon={faPlus} />
          </Button>
          <Button
            size="sm"
            variant="outlined"
            onClick={fetch_schemes}
            className="border-pm-primary text-pm-primary"
          >
            <FontAwesomeIcon icon={faArrowsRotate} />
          </Button>
        </div>
      </div>
      <ColorSchemeList schemes={schemes} />
    </div>
  );
}
