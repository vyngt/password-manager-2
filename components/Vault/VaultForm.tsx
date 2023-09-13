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
  open,
  item,
  item_manager,
}: {
  children: React.ReactNode;
  title: string;
  open: boolean;
  item: Item;
  item_manager: ItemManager;
}) => {
  return (
    <>
      <Dialog size="xs" open={open} handler={() => {}} className="shadow-none">
        <div className="mx-auto flex w-full max-w-[24rem] flex-col rounded-lg bg-pm-foreground p-6">
          <div className="mb-4 flex justify-center">
            <Typography variant="h4" className="!text-pm-primary">
              {title}
            </Typography>
          </div>
          <div className="mb-3 flex flex-col gap-4">
            <div className="relative h-10 w-full min-w-[200px]">
              <input
                className="pm-input peer text-pm-background"
                placeholder=" "
                value={item.name}
                onChange={(e) => item_manager.name(e.target.value)}
              />
              <label className="before:content[' '] after:content[' '] pm-input-label">
                Name
              </label>
            </div>

            <div className="relative h-10 w-full min-w-[200px]">
              <input
                className="pm-input peer text-pm-background"
                placeholder=" "
                value={item.url}
                onChange={(e) => item_manager.url(e.target.value)}
              />
              <label className="before:content[' '] after:content[' '] pm-input-label">
                URL
              </label>
            </div>

            <div className="relative h-10 w-full min-w-[200px]">
              <input
                className="pm-input peer text-pm-background"
                placeholder=" "
                value={item.username}
                onChange={(e) => item_manager.username(e.target.value)}
              />
              <label className="before:content[' '] after:content[' '] pm-input-label">
                Username
              </label>
            </div>

            <div className="relative h-10 w-full min-w-[200px]">
              <input
                className="pm-input peer text-pm-background"
                type="password"
                placeholder=" "
                value={item.password}
                onChange={(e) => item_manager.password(e.target.value)}
              />
              <label className="before:content[' '] after:content[' '] pm-input-label">
                Password
              </label>
            </div>
          </div>
          <div>{children}</div>
        </div>
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

  return (
    <>
      <Tooltip content="Add" className="bg-pm-foreground text-pm-background">
        <Button
          size="sm"
          onClick={handleOpen}
          className="bg-pm-primary text-pm-foreground"
        >
          <FontAwesomeIcon icon={faPlus} />
        </Button>
      </Tooltip>
      <VaultFormDialog
        title="Add Item"
        open={open}
        item={item}
        item_manager={item_manager}
      >
        <div className="flex gap-2">
          <Button
            size="sm"
            onClick={perform_submit}
            className="!bg-pm-primary !text-pm-foreground"
          >
            Add
          </Button>
          <Button
            size="sm"
            className="!bg-pm-foreground !text-pm-background"
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

export const VaultUpdateForm = ({
  item_id,
  update_ui,
}: {
  item_id: number;
  update_ui: (item: GItem) => void;
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

  const handleOpenEdit = async () => {
    try {
      const item: Item = await invoke("fetch_item", { id: item_id });
      set_item(item);
    } catch (e) {
    } finally {
      handleOpen();
    }
  };

  const perform_save = async () => {
    try {
      const result = await invoke("update_item", { ...item });
      if (result) {
        const new_item: GItem = await invoke("get_item", { id: item_id });
        update_ui(new_item);
      }
    } catch (e) {
    } finally {
      item_manager.clear();
      handleOpen();
    }
  };

  return (
    <>
      <Tooltip
        content="Edit Item"
        className="bg-pm-foreground text-pm-background"
      >
        <IconButton
          variant="text"
          className="bg-transparent text-pm-foreground"
          onClick={handleOpenEdit}
        >
          <FontAwesomeIcon icon={faPenToSquare} className="h-4 w-4" />
        </IconButton>
      </Tooltip>
      <VaultFormDialog
        title="Update Item"
        open={open}
        item={item}
        item_manager={item_manager}
      >
        <div className="flex gap-2">
          <Button
            size="sm"
            onClick={perform_save}
            className="!bg-pm-primary !text-pm-foreground"
          >
            Save
          </Button>
          <Button
            size="sm"
            className="!bg-pm-foreground !text-pm-background"
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
