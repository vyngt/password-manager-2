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
  ok,
  title,
  message,
}: {
  open: boolean;
  toggle: () => void;
  ok: boolean;
  title: string;
  message: string;
}) {
  return (
    <>
      <Dialog open={open} handler={toggle} size="md">
        <div className="mx-auto flex w-full flex-col justify-center gap-2 rounded-lg border border-pm-foreground bg-pm-background p-6">
          <div className="flex flex-col justify-center">
            <Typography variant="h4" className="self-center !text-pm-primary">
              {title}
            </Typography>
          </div>

          <Typography
            className={`self-center py-2 ${
              ok ? "!text-pm-success" : "!text-pm-danger"
            }`}
            variant="h5"
          >
            {message}
          </Typography>
          <div className="flex justify-center border-t border-pm-secondary pt-4">
            <Button
              variant="outlined"
              onClick={toggle}
              className="!border-pm-primary !text-pm-foreground"
            >
              Ok
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
