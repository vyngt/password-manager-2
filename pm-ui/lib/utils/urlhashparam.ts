"use client";

import { useEffect, useState } from "react";

interface ParamProxy {
  [key: string]: string;
}

const re = /([a-z]+)=([0-9]+)/;

const proxy: ParamProxy = {};
const get = (value: string) => proxy[value];
const getNumber = (value: string) => {
  const result = parseInt(proxy[value]);
  return Number.isNaN(result) ? 0 : result;
};

const ProxyManager = {
  get,
  getNumber,
};

export function useHashParam() {
  useEffect(() => {
    const sharp = window.location.hash.substring(1);
    if (sharp.length > 0) {
      for (const kv of sharp.split("&")) {
        if (re.test(kv)) {
          const [key, value] = kv.split("=");
          proxy[key] = value;
        }
      }
    }
  }, []);

  return ProxyManager;
}
