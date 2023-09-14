"use client";

import { useCurrentTheme } from "@/components/Theme";
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
  useCurrentTheme();

  return (
    <div className="relative h-full w-full">
      <Sidebar className="fixed h-full w-[16rem] border-r-2 border-solid border-pm-secondary" />
      <main className="ml-[16rem] mr-[2px]">{children}</main>
    </div>
  );
}
