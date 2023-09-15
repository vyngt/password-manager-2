import { faPenToSquare, faTrashCan } from "@fortawesome/free-regular-svg-icons";
import { IColorScheme } from "../Theme/types";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Chip, IconButton, Typography } from "@material-tailwind/react";
import { faPaintbrush } from "@fortawesome/free-solid-svg-icons";
import { useColorScheme, useColorSchemeManager } from "../Theme";

const ColorSchemeRow = ({
  color_scheme,
  children,
}: {
  color_scheme: IColorScheme;
  children?: React.ReactNode;
}) => {
  const { id } = useColorScheme();

  return (
    <tr style={{ backgroundColor: color_scheme.background }}>
      <td style={{ color: color_scheme.foreground }}>
        <div className="flex gap-2">
          <Typography variant="lead">{color_scheme.name}</Typography>
          {id == color_scheme.id && (
            <Chip
              className="border-pm-success text-pm-success"
              variant="outlined"
              value="Using"
            />
          )}
        </div>
      </td>
      <td>
        <div className="grid w-16 grid-cols-5 gap-1">
          {Object.entries(color_scheme).map(
            (d) =>
              !(d[0] == "name" || d[0] == "id") && (
                <div
                  key={`${color_scheme.id}-${d[0]}`}
                  className="h-3 w-3 rounded-md"
                  style={{ backgroundColor: d[1] }}
                ></div>
              ),
          )}
        </div>
      </td>
      <td>
        <div className="flex h-full justify-end gap-2">{children}</div>
      </td>
    </tr>
  );
};

export default function ColorSchemeList({
  schemes,
}: {
  schemes: Array<IColorScheme>;
}) {
  const manager = useColorSchemeManager();

  return (
    <div className="pm-color-scheme-table">
      <table>
        <tbody>
          {schemes.map((e) => (
            <ColorSchemeRow key={e.id} color_scheme={e}>
              <IconButton variant="text">
                <FontAwesomeIcon
                  onClick={() => manager.change_to(e.id)}
                  className="h-8 w-8"
                  icon={faPaintbrush}
                  style={{ color: e.primary }}
                />
              </IconButton>
              <IconButton variant="text">
                <FontAwesomeIcon
                  className="h-8 w-8"
                  icon={faPenToSquare}
                  style={{ color: e.foreground }}
                />
              </IconButton>
              <IconButton variant="text">
                <FontAwesomeIcon
                  className="h-8 w-8"
                  icon={faTrashCan}
                  style={{ color: e.danger }}
                />
              </IconButton>
            </ColorSchemeRow>
          ))}
        </tbody>
      </table>
    </div>
  );
}
