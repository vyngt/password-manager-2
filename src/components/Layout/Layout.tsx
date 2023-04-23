import { Sidebar } from "../Sidebar";
import type { ReactElement } from "react";
export const Layout = ({ children }: { children: ReactElement }) => {
  return (
    <main className="mx-auto flex w-full text-center">
      <Sidebar />
      {children}
    </main>
  );
};
