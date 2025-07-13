import { Typography } from "@/components/MaterialTailwind";

export function Section({
  name,
  children,
}: {
  name: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex w-full flex-col gap-2 p-4">
      <Typography className="text-primary" variant="h4">
        {name}
      </Typography>
      <hr className="w-full border-primary/40" />
      {children}
    </div>
  );
}
