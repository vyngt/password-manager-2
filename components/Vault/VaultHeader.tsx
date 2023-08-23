import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate } from "@fortawesome/free-solid-svg-icons";
import { Typography, Button, Tooltip } from "@material-tailwind/react";
import { VaultSearch } from "./VaultSearch";
import { IVaultHeaderManager, GItem } from "./models";
import { VaultCreationForm } from ".";

export default function VaultHeader({
  manager,
}: {
  manager: IVaultHeaderManager;
}) {
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
      <div className="flex gap-2">
        <VaultCreationForm refresh_method={manager.refresh} />

        <Tooltip content="Refresh">
          <Button size="sm" onClick={manager.refresh}>
            <FontAwesomeIcon icon={faArrowsRotate} />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}
