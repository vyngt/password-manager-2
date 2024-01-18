export interface SidebarItem {
  id: string;
  sequence: number;
}

export type SidebarChildComponent<T extends SidebarItem> = ({
  item,
}: {
  item: T;
}) => React.JSX.Element;
