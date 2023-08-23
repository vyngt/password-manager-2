import { Typography, IconButton, Tooltip } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faPenToSquare,
  faTrashCan,
  faCopy,
} from "@fortawesome/free-solid-svg-icons";
import { GItem, Operator } from "./models";

const TABLE_HEAD = ["Name", "URL", "Username", ""];

export const VaultRow = ({
  item,
  operator,
}: {
  item: GItem;
  operator: Operator<GItem>;
}) => {
  return (
    <tr className="hover:bg-blue-gray-100">
      <td className="pt-4">
        <div className="flex items-center gap-3">
          <div className="flex flex-col">
            <Typography
              variant="small"
              color="blue-gray"
              className="font-normal"
            >
              {item.name}
            </Typography>
          </div>
        </div>
      </td>
      <td className="pt-4">
        <div className="flex flex-col">
          <Typography variant="small" color="blue-gray" className="font-normal">
            {item.url}
          </Typography>
        </div>
      </td>
      <td className="pt-4">
        <Typography variant="small" color="blue-gray" className="font-normal">
          {item.username}
        </Typography>
      </td>
      <td className="pt-4">
        <Tooltip content="Copy Password">
          <IconButton variant="text" onClick={() => operator.copy(item)}>
            <FontAwesomeIcon icon={faCopy} className="h-4 w-4" />
          </IconButton>
        </Tooltip>
        <Tooltip content="Edit Item">
          <IconButton variant="text" onClick={() => operator.update(item)}>
            <FontAwesomeIcon icon={faPenToSquare} className="h-4 w-4" />
          </IconButton>
        </Tooltip>
        <Tooltip content="Delete Item">
          <IconButton variant="text" onClick={() => operator.delete(item)}>
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
        <tr>
          {TABLE_HEAD.map((head) => (
            <th
              key={head}
              className="cursor-pointer border-y border-blue-gray-100 bg-blue-gray-50/50 p-4 transition-colors hover:bg-blue-gray-50"
            >
              <Typography
                variant="small"
                color="blue-gray"
                className="flex items-center justify-between gap-2 font-normal leading-none opacity-70"
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
