"use client";

import { save, open } from "@tauri-apps/api/dialog";
import { invoke } from "@tauri-apps/api/tauri";

import { Typography, Button } from "@material-tailwind/react";
import { BackupDialog } from "@/components/Backup";
import { useState } from "react";

export default function Page() {
  const [dialog, setDialog] = useState({
    open: false,
    ok: false,
    title: "",
    message: "",
  });

  const dispatch_dialog = {
    toggle: () => {
      setDialog({ ...dialog, open: !dialog.open });
    },
    notify: ({
      ok,
      title,
      message,
    }: {
      ok: boolean;
      title: string;
      message: string;
    }) => {
      const new_dialog = {
        open: true,
        ok,
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
          ok: true,
          title: "Exported",
          message: "Data exported: OK",
        });
      } else {
        dispatch_dialog.notify({
          ok: false,
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
          ok: true,
          title: "Imported",
          message: "Data Import: OK",
        });
      } else {
        dispatch_dialog.notify({
          ok: false,
          title: "Error",
          message: "Wrong format or permission denied!!!",
        });
      }
    }
  };

  return (
    <div className="flex h-full w-full flex-col justify-center bg-transparent pt-4">
      <div className="mb-8 flex flex-col items-center justify-between gap-8">
        <Typography variant="h4" className="!text-pm-primary">
          Data Importer/Exporter
        </Typography>
        <Typography className="mt-1 font-normal">
          Backup or Restore Data, or maybe...? {"=))"}
        </Typography>
      </div>
      <div className="flex items-center justify-center gap-4">
        <Button
          className="!bg-pm-primary !text-pm-foreground"
          onClick={perform_export}
        >
          Export
        </Button>
        <Button
          className="!bg-pm-primary !text-pm-foreground"
          onClick={perform_import}
        >
          Import
        </Button>
      </div>
      <BackupDialog
        open={dialog.open}
        toggle={dispatch_dialog.toggle}
        ok={dialog.ok}
        title={dialog.title}
        message={dialog.message}
      />
    </div>
  );
}
