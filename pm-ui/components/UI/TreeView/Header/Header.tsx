export function Header({
  action,
  filter,
  paginator,
}: {
  action: React.ReactNode;
  filter?: React.ReactNode;
  paginator?: React.ReactNode;
}) {
  return (
    <div className="flex h-16 shrink-0 grow-0 border-b border-b-secondary bg-secondary/10">
      <div className="flex w-full flex-col justify-center">
        <div className="flex justify-around">
          <div className="flex gap-1 pl-4">{action}</div>
          <div className="flex grow justify-center px-2">
            {filter ? filter : ""}
          </div>
          <div className="pr-4">{paginator ? paginator : ""}</div>
        </div>
      </div>
    </div>
  );
}
