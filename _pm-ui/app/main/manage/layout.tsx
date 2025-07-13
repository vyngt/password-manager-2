import { ManageLayout } from "@/components/Features/Manage";

export default function Layout({ children }: { children: React.ReactNode }) {
  return <ManageLayout className="h-full">{children}</ManageLayout>;
}
