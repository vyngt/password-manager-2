export function TreeView({
  header,
  children,
}: {
  header?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-full w-full flex-col">
      {header ? header : ""}
      {children}
    </div>
  );
}
