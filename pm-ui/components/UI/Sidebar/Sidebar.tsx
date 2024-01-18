import { List } from "@/components/MaterialTailwind";
import type { SidebarItem, SidebarChildComponent } from "./define";
import classnames from "classnames";

interface SidebarProps extends React.HTMLAttributes<HTMLDivElement> {}

export function SidebarWrapper<T extends SidebarItem>(items: Array<T>) {
  const Wrapper = (Child: SidebarChildComponent<T>) => {
    const _Wrapper = ({ className, ...rest }: SidebarProps) => {
      const classes = classnames(
        "h-full overflow-x-scroll border-r border-r-secondary bg-secondary/10 text-foreground shadow-xl",
        className,
      );
      return (
        <div {...rest} className={classes}>
          <List className="min-w-0 gap-0 p-0">
            {items
              .sort((a, b) => a.sequence - b.sequence)
              .map((e) => (
                <Child key={e.id} item={e} />
              ))}
          </List>
        </div>
      );
    };
    return _Wrapper;
  };
  return Wrapper;
}
