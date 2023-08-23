import { Typography, Button } from "@material-tailwind/react";
import { VaultSearch } from "./VaultSearch";

export default function VaultHeader() {
  return (
    <div className="flex flex-col">
      <div className="mb-8 flex items-center justify-between gap-8">
        <div>
          <Typography variant="h5" color="blue-gray">
            Item list
          </Typography>
        </div>
        <div className="flex shrink-0 flex-col gap-2 sm:flex-row">
          <VaultSearch />
        </div>
      </div>
      <div className="flex">
        <Button size="sm">Add</Button>
      </div>
    </div>
  );
}
