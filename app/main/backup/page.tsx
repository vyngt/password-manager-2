"use client";

import { save, open } from "@tauri-apps/api/dialog";
import { invoke } from "@tauri-apps/api/tauri";

import {
  Card,
  CardHeader,
  CardFooter,
  CardBody,
  Typography,
  Button,
} from "@material-tailwind/react";
import { BackupDialog } from "@/components/Backup";
import { useState } from "react";

export default function Page() {
  const [dialog, setDialog] = useState({
    open: false,
    color: "",
    title: "",
    message: "",
  });

  const dispatch_dialog = {
    toggle: () => {
      setDialog({ ...dialog, open: !dialog.open });
    },
    notify: ({
      color,
      title,
      message,
    }: {
      color: string;
      title: string;
      message: string;
    }) => {
      const new_dialog = {
        open: true,
        color,
        title,
        message,
      };
      setDialog(new_dialog);
    },
  };

  const perform_export = async () => {
    const file_path = await save({
      filters: [
        {
          name: "Password Manager Data",
          extensions: ["json"],
        },
      ],
    });
    if (file_path) {
      const exported: boolean = await invoke("export", { path: file_path });
      if (exported) {
        dispatch_dialog.notify({
          color: "green",
          title: "Exported",
          message: "Data exported: OK",
        });
      } else {
        dispatch_dialog.notify({
          color: "red",
          title: "Error",
          message: "Something went wrong...",
        });
      }
    }
  };

  const perform_import = async () => {
    const file_path = await open({
      multiple: false,
      filters: [
        {
          name: "Password Manager Data",
          extensions: ["json"],
        },
      ],
    });
    if (file_path) {
      const imported: boolean = await invoke("import", { path: file_path });
      if (imported) {
        dispatch_dialog.notify({
          color: "green",
          title: "Imported",
          message: "Data Import: OK",
        });
      } else {
        dispatch_dialog.notify({
          color: "red",
          title: "Error",
          message: "Wrong format or permission denied!!!",
        });
      }
    }
  };

  return (
    <Card color="transparent" shadow={false} className="h-full w-full">
      <CardHeader floated={false} shadow={false} className="rounded-none">
        <div className="mb-8 flex flex-col items-center justify-between gap-8">
          <Typography variant="h4" color="blue-gray">
            Data Importer/Exporter
          </Typography>
          <Typography color="gray" className="mt-1 font-normal">
            Backup or Restore Data, or maybe...? {"=))"}
          </Typography>
        </div>
      </CardHeader>
      <CardBody className="flex justify-center items-center gap-4">
        <Button onClick={perform_export}>Export</Button>
        <Button onClick={perform_import}>Import</Button>
      </CardBody>
      <CardFooter>
        <BackupDialog
          open={dialog.open}
          toggle={dispatch_dialog.toggle}
          color={dialog.color}
          title={dialog.title}
          message={dialog.message}
        />
      </CardFooter>
    </Card>
  );
}
