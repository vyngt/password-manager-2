import { faPenToSquare, faTrashCan } from "@fortawesome/free-regular-svg-icons";
import { IColorScheme } from "../Theme/types";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Chip, IconButton, Typography } from "@material-tailwind/react";
import { faPaintbrush } from "@fortawesome/free-solid-svg-icons";
import { useColorScheme, useColorSchemeManager } from "../Theme";
import { useManager } from "./Context";
import { useEffect } from "react";

const ColorSchemeRow = ({ color_scheme }: { color_scheme: IColorScheme }) => {
  const { id } = useColorScheme();
  const context = useManager();
  const manager = useColorSchemeManager();

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
        <div className="pm-color-scheme-table-row-operator">
          <IconButton variant="text">
            <FontAwesomeIcon
              onClick={() => manager.change_to(color_scheme.id)}
              icon={faPaintbrush}
              style={{ color: color_scheme.primary }}
            />
          </IconButton>
          <IconButton variant="text">
            <FontAwesomeIcon
              icon={faPenToSquare}
              style={{ color: color_scheme.foreground }}
            />
          </IconButton>
          <IconButton
            variant="text"
            onClick={() => {
              if (id == color_scheme.id) {
                return;
              }
              context.remove(color_scheme.id);
            }}
            disabled={color_scheme.id == id}
          >
            <FontAwesomeIcon
              icon={faTrashCan}
              style={{ color: color_scheme.danger }}
            />
          </IconButton>
        </div>
      </td>
    </tr>
  );
};

export default function ColorSchemeTable() {
  const context = useManager();

  useEffect(() => {
    context.reload();
  }, []);

  return (
    <div className="pm-color-scheme-table">
      <table>
        <tbody>
          {context.schemes.map((e) => (
            <ColorSchemeRow key={e.id} color_scheme={e} />
          ))}
        </tbody>
      </table>
    </div>
  );
}
