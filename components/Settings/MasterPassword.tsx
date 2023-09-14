"use client";
import { faCancel, faFloppyDisk } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  Typography,
  Input,
  Button,
  Dialog,
  DialogBody,
  DialogHeader,
  DialogFooter,
} from "@material-tailwind/react";
import { useState } from "react";

import { invoke } from "@tauri-apps/api/tauri";

const SetMasterPasswordDialog = ({
  open,
  toggle,
}: {
  open: boolean;
  toggle: () => void;
}) => {
  const [password, setPassword] = useState("");

  const clear = () => {
    setPassword("");
    toggle();
  };

  const save = async () => {
    try {
      const result = await invoke("change_master_password", {
        password: password,
      });
    } catch (e) {
    } finally {
      clear();
    }
  };

  return (
    <Dialog size="xs" open={open} handler={clear}>
      <div className="mx-auto flex w-full flex-col rounded-lg border border-pm-foreground bg-pm-background p-6">
        <div className="mb-4 flex justify-center">
          <Typography variant="h4" className="!text-pm-primary">
            Set New Master Password
          </Typography>
        </div>
        <div className="mb-3 flex flex-col gap-4">
          <div className="relative h-10">
            <input
              className="pm-input peer text-pm-foreground"
              type="password"
              placeholder=" "
              value={password}
              onChange={(ev) => setPassword(ev.target.value)}
            />
            <label className="before:content[' '] after:content[' '] pm-input-label">
              Password
            </label>
          </div>
        </div>
        <div className="flex gap-3">
          <Button
            variant="outlined"
            onClick={save}
            className="!border-pm-primary !text-pm-foreground"
          >
            <FontAwesomeIcon icon={faFloppyDisk} />
          </Button>
          <Button
            variant="outlined"
            onClick={clear}
            className="!border-pm-primary !text-pm-foreground"
          >
            <FontAwesomeIcon icon={faCancel} />
          </Button>
        </div>
      </div>
    </Dialog>
  );
};

export default function MasterPassword() {
  const [open, setOpen] = useState(false);

  const toggle = () => setOpen((cur) => !cur);

  return (
    <div className="mb-5 h-full w-full">
      <Typography variant="h6" className="border-b-2 border-pm-secondary pl-1">
        Master Password
      </Typography>
      <div className="relative flex w-full gap-3 pl-8 pt-4">
        <Button
          size="sm"
          onClick={toggle}
          variant="outlined"
          className="!border-pm-primary !text-pm-foreground"
        >
          New Master Password
        </Button>
        <SetMasterPasswordDialog open={open} toggle={toggle} />
      </div>
    </div>
  );
}
