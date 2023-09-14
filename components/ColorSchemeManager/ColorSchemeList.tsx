import { IColorScheme } from "../Theme/types";

const ColorSchemeRow = ({
  color_scheme,
  children,
}: {
  color_scheme: IColorScheme;
  children?: React.ReactNode;
}) => {
  return (
    <tr>
      <td>{color_scheme.name}</td>
      <td></td>
      <td>{children}</td>
    </tr>
  );
};

export default function ColorSchemeList({
  schemes,
}: {
  schemes: Array<IColorScheme>;
}) {
  const color_schemes = [
    { id: 1, name: "Default", color: "Heh" },
    { id: 2, name: "Scheme 2", color: "Heh" },
    { id: 3, name: "Scheme 3", color: "Heh" },
    { id: 4, name: "Scheme 4", color: "Heh" },
    { id: 5, name: "Scheme 5", color: "Heh" },
    { id: 6, name: "Scheme 6 ", color: "Heh" },
    { id: 7, name: "Scheme 7", color: "Heh" },
    { id: 8, name: "Scheme 8", color: "Heh" },
    { id: 9, name: "Scheme 9", color: "Heh" },
    { id: 10, name: "Scheme 10", color: "Heh" },
    { id: 11, name: "Scheme 11", color: "Heh" },
    { id: 12, name: "Scheme 12", color: "Heh" },
  ];

  return (
    <div className="pm-color-scheme-table">
      <table>
        <tbody>
          {schemes.map((e) => (
            <ColorSchemeRow key={e.id} color_scheme={e}></ColorSchemeRow>
          ))}
        </tbody>
      </table>
    </div>
  );
}
