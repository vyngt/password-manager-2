"use client";

import { useState, useEffect } from "react";
import { hex2rgb } from "@/lib/utils";

export default function ThemeForm() {
  const [hex, setHex] = useState("#ffffff");
  const [rgb, setRGB] = useState("0 0 0");

  useEffect(() => {
    setRGB(hex2rgb(hex));
  }, [hex]);

  return (
    <div>
      <div>RGB: {rgb}</div>
      <input
        type="color"
        value={hex}
        onChange={(ev) => setHex(ev.target.value)}
      />
    </div>
  );
}
