"use client";

import { createTreeProvider, createTreeReducer } from "../UI/TreeView";
import { ColorScheme } from "./define";

const reducer = createTreeReducer<ColorScheme>();
const { Provider, hook } = createTreeProvider<ColorScheme>({ reducer });

export const { useTree, useTreeState, useTreeDispatch } = hook;
export { Provider };
