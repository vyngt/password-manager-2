import { LayoutWrapper } from "@/components/UI/Sidebar";
import { Sidebar } from "./Sidebar";

export const ManageLayout = LayoutWrapper(
  <Sidebar className="app-none-scrollbar h-full border-r border-r-secondary/10 bg-secondary/5 text-foreground" />,
);
