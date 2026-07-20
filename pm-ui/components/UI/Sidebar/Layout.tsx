import classnames from "classnames";

interface LayoutProps extends React.HTMLAttributes<HTMLDivElement> {}

export function LayoutWrapper(sidebar: React.ReactNode) {
  const Wrapper = ({ children, className, ...rest }: LayoutProps) => {
    const classes = classnames("flex w-full", className);
    return (
      <div {...rest} className={classes}>
        {sidebar}
        <div className="app-scrollbar flex-1 overflow-y-auto">{children}</div>
      </div>
    );
  };
  return Wrapper;
}
