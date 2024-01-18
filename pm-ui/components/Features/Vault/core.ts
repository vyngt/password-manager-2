"use client";

import { createTreeProvider, createTreeReducer } from "../../UI/TreeView";
import { Item } from "./define";

const reducer = createTreeReducer<Item>();
const { Provider, hook } = createTreeProvider<Item>({ reducer });

export const { useTree, useTreeState, useTreeDispatch } = hook;
export { Provider };
