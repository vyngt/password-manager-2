export function Form({ children }: { children: React.ReactNode }) {
  return (
    <form className="m-3 flex flex-col gap-4 rounded-md border border-secondary/60 p-5">
      {children}
    </form>
  );
}
