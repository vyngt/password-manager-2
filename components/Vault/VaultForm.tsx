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

import { GItem, Item, ItemManager } from "./models";

import { invoke } from "@tauri-apps/api/tauri";

const VaultFormDialog = ({
  children,
  title,
  toggler,
  item,
  item_manager,
}: {
  children: React.ReactNode;
  title: string;
  toggler: {
    open: boolean;
    handleOpen: () => void;
  };
  item: Item;
  item_manager: ItemManager;
}) => {
  return (
    <>
      <Dialog
        size="xs"
        open={toggler.open}
        handler={() => {}}
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
            <Input
              value={item.name}
              onChange={(e) => item_manager.name(e.target.value)}
              label="Name"
              size="lg"
              crossOrigin={""}
            />
            <Input
              value={item.url}
              onChange={(e) => item_manager.url(e.target.value)}
              label="URL"
              size="lg"
              crossOrigin={""}
            />
            <Input
              value={item.username}
              onChange={(e) => item_manager.username(e.target.value)}
              label="Username"
              size="lg"
              crossOrigin={""}
            />
            <Input
              value={item.password}
              onChange={(e) => item_manager.password(e.target.value)}
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

export const VaultCreationForm = ({
  refresh_method,
}: {
  refresh_method: () => void;
}) => {
  const [open, setOpen] = React.useState(false);
  const handleOpen = () => setOpen((cur) => !cur);

  const [item, set_item] = React.useState<Item>({
    id: 0,
    name: "",
    password: "",
    url: "",
    username: "",
  });

  const item_manager: ItemManager = {
    name: (name: string) => {
      set_item({ ...item, name: name });
    },
    url: (url: string) => {
      set_item({ ...item, url: url });
    },
    username: (username: string) => {
      set_item({ ...item, username: username });
    },
    password: (password: string) => {
      set_item({ ...item, password: password });
    },
    clear: () => {
      set_item({ id: 0, name: "", password: "", url: "", username: "" });
    },
  };

  const perform_submit = async () => {
    const new_item = {
      name: item.name,
      url: item.url,
      username: item.username,
      password: item.password,
    };
    try {
      const result = await invoke("create_item", { ...new_item });
      if (result) {
        refresh_method();
      }
    } catch (e) {
    } finally {
      item_manager.clear();
    }
  };

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
      <VaultFormDialog
        title="Add Item"
        toggler={toggler}
        item={item}
        item_manager={item_manager}
      >
        <div className="flex gap-2">
          <Button size="sm" onClick={perform_submit}>
            Add
          </Button>
          <Button
            size="sm"
            color="white"
            onClick={() => {
              item_manager.clear();
              handleOpen();
            }}
          >
            Cancel
          </Button>
        </div>
      </VaultFormDialog>
    </>
  );
};

// ----------------------------------------------------------------
// ----------------------------------------------------------------

export const VaultUpdateForm = () => {
  const [open, setOpen] = React.useState(false);
  const handleOpen = () => setOpen((cur) => !cur);

  const [item, set_item] = React.useState<Item>({
    id: 0,
    name: "",
    password: "",
    url: "",
    username: "",
  });

  const item_manager: ItemManager = {
    name: (name: string) => {
      set_item({ ...item, name: name });
    },
    url: (url: string) => {
      set_item({ ...item, url: url });
    },
    username: (username: string) => {
      set_item({ ...item, username: username });
    },
    password: (password: string) => {
      set_item({ ...item, password: password });
    },
    clear: () => {
      set_item({ id: 0, name: "", password: "", url: "", username: "" });
    },
  };

  const toggler = {
    open,
    handleOpen,
  };

  return (
    <>
      <Tooltip content="Edit Item">
        <IconButton variant="text" onClick={handleOpen}>
          <FontAwesomeIcon icon={faPenToSquare} className="h-4 w-4" />
        </IconButton>
      </Tooltip>
      <VaultFormDialog
        title="Update Item"
        toggler={toggler}
        item={item}
        item_manager={item_manager}
      >
        <div className="flex gap-2">
          <Button size="sm" onClick={handleOpen}>
            Save
          </Button>
          <Button
            size="sm"
            color="white"
            onClick={() => {
              handleOpen();
              item_manager.clear();
            }}
          >
            Cancel
          </Button>
        </div>
      </VaultFormDialog>
    </>
  );
};
