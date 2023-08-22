import { Sidebar } from "@/components/Sidebar";

export const metadata = {
  title: "Main",
  description: "Main App Window",
};

export default function MainLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="w-full h-full flex">
      <Sidebar />
      <main className="w-full h-full">{children}</main>
    </div>
  );
}
