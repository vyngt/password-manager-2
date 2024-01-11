"use client";
import Link from "next/link";
import { useEffect, useReducer, useCallback, useState } from "react";
import { useRouter } from "next/navigation";
import { invoke } from "@tauri-apps/api/tauri";

import { useHashParam } from "@/lib/utils";
import { Input } from "@/components/UI";
import { IconButton } from "@/components/MaterialTailwind";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faTrashCan,
  faArrowLeft,
  faFloppyDisk,
} from "@fortawesome/free-solid-svg-icons";

import { ItemCreate, ItemInDB } from "@/components/Vault/define";

const initItem: ItemCreate = {
  password: "",
  name: "",
  url: "",
  username: "",
};

interface ItemFormAction {
  type: keyof ItemCreate;
  payload: string;
}

interface ItemLoadAction {
  type: "load";
  payload: ItemCreate;
}

const reducer = (
  state: ItemCreate,
  action: ItemFormAction | ItemLoadAction,
) => {
  switch (action.type) {
    case "load":
      return { ...state, ...action.payload };
    default:
      return { ...state, [action.type]: action.payload };
  }
};

function FormHeader({
  actions,
  others,
}: {
  actions?: React.ReactNode;
  others?: React.ReactNode;
}) {
  return (
    <div className="flex h-16 grow-0 border-b border-b-secondary bg-secondary/10">
      <div className="flex w-full flex-col justify-center">
        <div className="flex justify-around">
          <div className="flex gap-1 pl-4">{actions ? actions : ""}</div>
          <div className="flex grow justify-center px-2">
            {others ? others : ""}
          </div>
        </div>
      </div>
    </div>
  );
}

export default function VaultItemFormPage() {
  const hashParam = useHashParam();
  const [state, dispatch] = useReducer(reducer, initItem);
  const [itemId, setItemId] = useState(0);

  const router = useRouter();

  const performGetItem = useCallback(
    async (_id: number) => {
      const result: ItemInDB = await invoke("get_item", { id: _id });
      if (result) {
        const { id, ...rest } = result;
        dispatch({ type: "load", payload: rest });
      }
    },
    [dispatch],
  );

  const performAfterCreated = async (item?: ItemInDB) => {
    if (item) {
      router.replace(`/main/vault/form#id=${item.id}`);
      setItemId(item.id);
    }
  };

  const performAfterUpdated = async (item?: ItemInDB) => {
    // console.log("Updated", item);
  };

  const performSave = async () => {
    const id = hashParam.getNumber("id");
    if (id === 0) {
      const result: ItemInDB = await invoke("create_item", { data: state });
      await performAfterCreated(result);
    } else if (id > 0) {
      const input = {
        ...state,
        id: id,
      };
      const result: ItemInDB = await invoke("update_item", { data: input });
      await performAfterUpdated(result);
    }
  };

  const performDelete = async () => {
    const id = hashParam.getNumber("id");

    if (id > 0) {
      const result: boolean = await invoke("delete_item", { id: id });
      if (result) router.push("/main/vault/");
    }
  };

  useEffect(() => {
    const id = hashParam.getNumber("id");
    if (id > 0) {
      setItemId(id);
    }
  }, []);

  useEffect(() => {
    if (itemId > 0) {
      performGetItem(itemId);
    }
  }, [itemId, performGetItem]);

  return (
    <div className="flex h-full w-full flex-col gap-3">
      <FormHeader
        actions={
          <>
            <Link href="/main/vault/">
              <IconButton
                variant="filled"
                className="bg-primary/70 text-foreground"
              >
                <FontAwesomeIcon icon={faArrowLeft} />
              </IconButton>
            </Link>
            <IconButton
              onClick={performSave}
              variant="outlined"
              className="border-secondary text-secondary focus:ring-secondary/30"
            >
              <FontAwesomeIcon icon={faFloppyDisk} />
            </IconButton>
            <IconButton
              onClick={performDelete}
              variant="outlined"
              className={`border-secondary text-secondary focus:ring-secondary/30 ${
                itemId ? "" : "hidden"
              }`}
            >
              <FontAwesomeIcon icon={faTrashCan} />
            </IconButton>
          </>
        }
      />
      <form className="m-3 flex flex-col gap-4 rounded-md border border-secondary/60 p-5">
        <Input
          color="primary"
          label="Name"
          value={state.name}
          onChange={(ev) =>
            dispatch({ type: "name", payload: ev.currentTarget.value })
          }
        />
        <Input
          color="primary"
          label="URL"
          value={state.url}
          onChange={(ev) =>
            dispatch({ type: "url", payload: ev.currentTarget.value })
          }
        />
        <Input
          color="primary"
          label="Username"
          value={state.username}
          onChange={(ev) =>
            dispatch({ type: "username", payload: ev.currentTarget.value })
          }
        />
        <Input
          color="primary"
          label="Password"
          type="password"
          value={state.password}
          onChange={(ev) =>
            dispatch({ type: "password", payload: ev.currentTarget.value })
          }
        />
      </form>
    </div>
  );
}
