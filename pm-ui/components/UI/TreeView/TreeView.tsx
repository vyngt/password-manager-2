export function TreeView({
  header,
  table,
}: {
  header?: React.ReactNode;
  table: React.ReactNode;
}) {
  return (
    <div className="flex h-full w-full flex-col">
      {header ? header : ""}
      {table}
    </div>
  );
}
