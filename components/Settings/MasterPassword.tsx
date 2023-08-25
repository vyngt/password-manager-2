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
    <Dialog open={open} handler={clear}>
      <DialogHeader>
        <Typography variant="h5" color="blue-gray">
          Set New Master Password
        </Typography>
      </DialogHeader>
      <DialogBody divider className="grid place-items-center gap-4">
        <Input
          crossOrigin={""}
          type="password"
          label="Password"
          value={password}
          onChange={(ev) => setPassword(ev.target.value)}
        />
      </DialogBody>
      <DialogFooter className="space-x-2">
        <Button variant="gradient" onClick={save}>
          <FontAwesomeIcon icon={faFloppyDisk} />
        </Button>
        <Button variant="gradient" onClick={clear}>
          <FontAwesomeIcon icon={faCancel} />
        </Button>
      </DialogFooter>
    </Dialog>
  );
};

export default function MasterPassword() {
  const [open, setOpen] = useState(false);

  const toggle = () => setOpen((cur) => !cur);

  return (
    <>
      <Typography variant="h6" className="bg-brown-100 pl-4">
        Master Password
      </Typography>
      <br />
      <div className="pl-8 relative flex w-full max-w-[24rem] gap-3">
        <Button size="sm" onClick={toggle}>
          New Master Password
        </Button>
        <SetMasterPasswordDialog open={open} toggle={toggle} />
      </div>
    </>
  );
}
