import { Typography, IconButton, Tooltip } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faTrashCan, faCopy } from "@fortawesome/free-solid-svg-icons";
import { GItem, Operator } from "./models";
import { VaultUpdateForm } from "./VaultForm";

const TABLE_HEAD = ["Name", "URL", "Username", ""];

export const VaultRow = ({
  item,
  operator,
}: {
  item: GItem;
  operator: Operator<GItem>;
}) => {
  return (
    <tr className="opacity-70 hover:opacity-100">
      <td className="pt-4">
        <div className="flex items-center gap-3">
          <div className="flex flex-col">
            <Typography variant="small" className="pl-4 font-normal">
              {item.name}
            </Typography>
          </div>
        </div>
      </td>
      <td className="pt-4">
        <div className="flex flex-col">
          <Typography variant="small" className="pl-4 font-normal">
            {item.url}
          </Typography>
        </div>
      </td>
      <td className="pt-4">
        <Typography variant="small" className="pl-4 font-normal">
          {item.username}
        </Typography>
      </td>
      <td className="pt-4">
        <Tooltip
          content="Copy Password"
          className="bg-pm-foreground text-pm-background"
        >
          <IconButton
            variant="text"
            className="bg-transparent text-pm-foreground"
            onClick={() => operator.copy(item)}
          >
            <FontAwesomeIcon icon={faCopy} className="h-4 w-4" />
          </IconButton>
        </Tooltip>
        <VaultUpdateForm item_id={item.id} update_ui={operator.update} />
        <Tooltip
          content="Delete Item"
          className="bg-pm-foreground text-pm-background"
        >
          <IconButton
            variant="text"
            className="bg-transparent text-pm-foreground"
            onClick={() => operator.delete(item)}
          >
            <FontAwesomeIcon icon={faTrashCan} className="h-4 w-4" />
          </IconButton>
        </Tooltip>
      </td>
    </tr>
  );
};

export default function VaultTable({
  items,
  operator,
}: {
  items: GItem[];
  operator: Operator<GItem>;
}) {
  return (
    <table className="mt-4 w-full min-w-max table-auto text-left">
      <thead>
        <tr className="border-y-2 border-pm-secondary text-pm-primary">
          {TABLE_HEAD.map((head) => (
            <th key={head} className="border-y p-4 uppercase">
              <Typography
                variant="small"
                className="flex items-center justify-between gap-2 font-normal leading-none"
              >
                {head}
              </Typography>
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {items.map((e) => (
          <VaultRow key={e.id} item={e} operator={operator} />
        ))}
      </tbody>
    </table>
  );
}
