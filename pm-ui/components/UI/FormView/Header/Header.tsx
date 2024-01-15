export function Header({
  actions,
  others,
}: {
  actions?: React.ReactNode;
  others?: React.ReactNode;
}) {
  return (
    <div className="flex h-16 grow-0 border-b border-b-secondary bg-secondary/10">
      <div className="flex w-full flex-col justify-center">
        <div className="flex justify-around">
          <div className="flex gap-1 pl-4">{actions ? actions : ""}</div>
          <div className="flex grow justify-center px-2">
            {others ? others : ""}
          </div>
        </div>
      </div>
    </div>
  );
}
