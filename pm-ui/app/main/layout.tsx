import { PanelLayout } from "@/components/Panel";

export default function MainLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <PanelLayout className="root-layout relative flex w-full">
      {children}
    </PanelLayout>
  );
}
