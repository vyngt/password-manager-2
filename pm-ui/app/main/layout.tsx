import { Sidebar } from "@/components/Panel";

export default function MainLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="root-layout relative flex w-full">
      <Sidebar className="app-none-scrollbar h-full w-32" />
      <div className="app-scrollbar flex-1 overflow-y-auto">{children}</div>
    </div>
  );
}
