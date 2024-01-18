import { Sidebar } from "@/components/Panel";

export default function ManageLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-full">
      <Sidebar className="app-none-scrollbar h-full w-16" />
      <div className="app-scrollbar flex-1 overflow-y-auto">{children}</div>
    </div>
  );
}
