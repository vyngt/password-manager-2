import React from "react";
import {
  Button,
  Dialog,
  DialogHeader,
  DialogBody,
  DialogFooter,
  Typography,
} from "@material-tailwind/react";

export function BackupDialog({
  open,
  toggle,
  color,
  title,
  message,
}: {
  open: boolean;
  toggle: () => void;
  color: string;
  title: string;
  message: string;
}) {
  return (
    <>
      <Dialog open={open} handler={toggle}>
        <DialogHeader>
          <Typography variant="h5" color="blue-gray">
            {title}
          </Typography>
        </DialogHeader>
        <DialogBody divider className="grid place-items-center gap-4">
          <Typography color={color} variant="h4">
            {message}
          </Typography>
        </DialogBody>
        <DialogFooter className="space-x-2">
          <Button variant="gradient" onClick={toggle}>
            Ok
          </Button>
        </DialogFooter>
      </Dialog>
    </>
  );
}
