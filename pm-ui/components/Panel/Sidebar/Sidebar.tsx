import type { ISidebarItem } from "./items";
import { SidebarManager } from "./items";
import { Item } from "./Item";
import { List } from "@/components/MaterialTailwind";

const SidebarItem = ({ item }: { item: ISidebarItem }) => {
  if (item.type == "action") return <Item item={item} />;
  else return <></>;
};

export function Sidebar({
  className,
  ...rest
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={`h-full border-r border-r-secondary bg-secondary/10 text-foreground shadow-xl ${className}`}
      {...rest}
    >
      <List className="min-w-0 gap-0 p-0">
        {SidebarManager.all().map((e) => (
          <SidebarItem key={e.id} item={e} />
        ))}
      </List>
    </div>
  );
}
