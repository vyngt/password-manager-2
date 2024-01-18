import { LayoutWrapper } from "@/components/UI/Sidebar";
import { Sidebar } from "./Sidebar";

export const PanelLayout = LayoutWrapper(
  <Sidebar className="app-none-scrollbar h-full w-16 border-r border-r-secondary bg-secondary/10 text-foreground" />,
);
