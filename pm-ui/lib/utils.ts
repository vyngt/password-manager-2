"use client";

import { useEffect, useState } from "react";

interface ParamProxy {
  [key: string]: string;
}

const re = /([a-z]+)=([0-9]+)/;

export function useHashParam() {
  const [state, setState] = useState<ParamProxy>({});

  const get = (value: string) => state[value];
  const getNumber = (value: string) => {
    const result = parseInt(state[value]);
    return Number.isNaN(result) ? 0 : result;
  };

  useEffect(() => {
    const sharp = window.location.hash.substring(1);
    if (sharp.length > 0) {
      for (const kv of sharp.split("&")) {
        if (re.test(kv)) {
          const [key, value] = kv.split("=");
          state[key] = value;
        }
      }
    }
    setState({ ...state });
  }, []);

  return {
    get,
    getNumber,
  };
}
