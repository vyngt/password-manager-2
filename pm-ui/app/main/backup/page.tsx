"use client";

import { invoke } from "@tauri-apps/api/tauri";
import { save, open } from "@tauri-apps/api/dialog";
import { useState, useEffect } from "react";
import { Typography, Button } from "@/components/MaterialTailwind";

function Importer() {
  const [loading, setLoading] = useState(false);
  const [ok, setOk] = useState<boolean | null>(null);

  const performImport = async () => {
    const filePath = await open({
      multiple: false,
      filters: [
        {
          name: "Password Manager Data",
          extensions: ["json"],
        },
      ],
    });
    if (filePath) {
      setLoading(true);
      const imported = await invoke<boolean>("import_vault", {
        path: filePath,
      });
      setOk(imported);
      setLoading(false);
    }
  };
  return (
    <>
      <Typography className="text-secondary" variant="lead">
        Import data
      </Typography>
      <Typography className="text-foreground">
        Using file like .json to import into your vault
      </Typography>
      <Button
        className="bg-primary/50 text-foreground"
        onClick={performImport}
        loading={loading}
      >
        Backup
      </Button>
    </>
  );
}

function Exporter() {
  const [loading, setLoading] = useState(false);
  const [ok, setOk] = useState<boolean | null>(null);

  const performExport = async () => {
    const filePath = await save({
      filters: [
        {
          name: "Password Manager Data",
          extensions: ["json"],
        },
      ],
    });
    if (filePath) {
      setLoading(true);
      const exported = await invoke<boolean>("export_vault", {
        path: filePath,
      });
      setOk(exported);
      setLoading(false);
    }
  };

  return (
    <>
      <Typography className="text-secondary" variant="lead">
        Export data
      </Typography>
      <Typography className="text-foreground">
        Export your vault to transfer to another machine...
        <i className="text-warning">(Data exported will be plaintext)</i>
      </Typography>
      <Button
        className="bg-secondary/50 text-foreground"
        onClick={performExport}
        loading={loading}
      >
        Create
      </Button>
    </>
  );
}

export default function Page() {
  return (
    <div className="p-4">
      <div className="mb-2">
        <Typography className="text-primary" variant="h3">
          Data Backup
        </Typography>
      </div>

      <div className="mt-3">
        <Importer />
      </div>

      <div className="mt-3">
        <Exporter />
      </div>
    </div>
  );
}
