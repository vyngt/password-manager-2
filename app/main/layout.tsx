"use client";

import { useColorScheme } from "@/components/Theme";
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
  const { background, foreground } = useColorScheme();
  return (
    <div
      className="w-full h-full flex"
      style={{ backgroundColor: background, color: foreground }}
    >
      <Sidebar className="h-full w-[20rem]" />
      <main className="w-full h-full">{children}</main>
    </div>
  );
}
