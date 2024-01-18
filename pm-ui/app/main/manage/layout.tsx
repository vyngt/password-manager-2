import { PanelLayout } from "@/components/Panel";

export default function ManageLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <PanelLayout className="h-full">{children}</PanelLayout>;
}
