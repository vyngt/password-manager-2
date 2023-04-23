import { VTable } from "../components/Vault";
import { Sidebar } from "@/components/Sidebar";

export default function Vault() {
  return (
    <main className="mx-auto flex w-full flex-wrap text-center">
      <Sidebar />
      <VTable />
    </main>
  );
}
