import React from "react";
import {
  Button,
  Dialog,
  Card,
  CardHeader,
  CardBody,
  CardFooter,
  Typography,
  Input,
  Tooltip,
  IconButton,
} from "@material-tailwind/react";

import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPenToSquare, faPlus } from "@fortawesome/free-solid-svg-icons";

const VaultFormDialog = ({
  children,
  title,
  toggler,
}: {
  children: React.ReactNode;
  title: string;
  toggler: {
    open: boolean;
    handleOpen: () => void;
  };
}) => {
  return (
    <>
      <Dialog
        size="xs"
        open={toggler.open}
        handler={toggler.handleOpen}
        className="bg-transparent shadow-none"
      >
        <Card className="mx-auto w-full max-w-[24rem]">
          <CardHeader
            variant="gradient"
            className="mb-4 grid h-28 place-items-center bg-black"
          >
            <Typography variant="h4" color="white">
              {title}
            </Typography>
          </CardHeader>
          <CardBody className="flex flex-col gap-4">
            <Input label="Name" size="lg" crossOrigin={""} />
            <Input label="URL" size="lg" crossOrigin={""} />
            <Input label="Username" size="lg" crossOrigin={""} />
            <Input
              label="Password"
              type="password"
              size="lg"
              crossOrigin={""}
            />
          </CardBody>
          <CardFooter className="pt-0">{children}</CardFooter>
        </Card>
      </Dialog>
    </>
  );
};

export const VaultCreationForm = () => {
  const [open, setOpen] = React.useState(false);
  const handleOpen = () => setOpen((cur) => !cur);

  const toggler = {
    open,
    handleOpen,
  };

  return (
    <>
      <Tooltip content="Add">
        <Button size="sm" onClick={handleOpen}>
          <FontAwesomeIcon icon={faPlus} />
        </Button>
      </Tooltip>
      <VaultFormDialog title="Add Item" toggler={toggler}>
        <Button size="sm" onClick={handleOpen}>
          Add
        </Button>
      </VaultFormDialog>
    </>
  );
};

export const VaultUpdateForm = () => {
  const [open, setOpen] = React.useState(false);
  const handleOpen = () => setOpen((cur) => !cur);

  const toggler = {
    open,
    handleOpen,
  };

  return (
    <>
      <IconButton variant="text" onClick={handleOpen}>
        <FontAwesomeIcon icon={faPenToSquare} className="h-4 w-4" />
      </IconButton>
      <VaultFormDialog title="Update Item" toggler={toggler}>
        <div className="flex gap-2">
          <Button size="sm" onClick={handleOpen}>
            Save
          </Button>
          <Button size="sm" color="white" onClick={handleOpen}>
            Cancel
          </Button>
        </div>
      </VaultFormDialog>
    </>
  );
};
