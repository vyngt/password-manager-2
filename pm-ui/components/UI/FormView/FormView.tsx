export function FormView({
  children,
  header,
}: {
  children: React.ReactNode;
  header: React.ReactNode;
}) {
  return (
    <div className="flex h-full w-full flex-col gap-3">
      {header}
      {children}
    </div>
  );
}
