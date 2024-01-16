export function FormView({ children }: { children: React.ReactNode }) {
  return <div className="flex h-full w-full flex-col gap-3">{children}</div>;
}
