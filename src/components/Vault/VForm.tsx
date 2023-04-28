import { ChangeEvent, MouseEvent, ReactElement, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useState, forwardRef } from "react";
import { NewItem, Item, FormItem, ItemInputUpdater } from "@/models";
import { TextInput, PasswordInput } from "./Input";

export const VForm = <T extends FormItem>({
  state,
  manager,
  children,
}: {
  state: T;
  manager: ItemInputUpdater<ChangeEvent<HTMLInputElement>>;
  children: ReactElement;
}) => {
  return (
    <form className="v-center">
      <TextInput
        label={"Name"}
        name={"name"}
        state={state.name}
        handler={manager.update_name}
      />
      <TextInput
        label={"URL"}
        name={"url"}
        state={state.url}
        handler={manager.update_url}
      />
      <TextInput
        label={"Username"}
        name={"username"}
        state={state.username}
        handler={manager.update_username}
      />
      <PasswordInput
        label={"Password"}
        name={"password"}
        state={state.password}
        handler={manager.update_password}
      />
      {children}
    </form>
  );
};

export const VCreateForm = () => {
  const [item, set_item] = useState<NewItem>({
    name: "",
    url: "",
    username: "",
    password: "",
  });

  const manager: ItemInputUpdater<ChangeEvent<HTMLInputElement>> = {
    update_name: (e) => {
      set_item({ ...item, name: e.target.value });
    },
    update_url: (e) => {
      set_item({ ...item, url: e.target.value });
    },
    update_username: (e) => {
      set_item({ ...item, username: e.target.value });
    },
    update_password: (e) => {
      set_item({ ...item, password: e.target.value });
    },
  };
  const reset_value = () => {
    set_item({
      name: "",
      url: "",
      username: "",
      password: "",
    });
  };
  const perform_create = async (e: MouseEvent<HTMLButtonElement>) => {
    const button = e.currentTarget;
    button.disabled = true;
    await invoke("create_item", { ...item });

    reset_value();
    setTimeout(() => {
      button.disabled = false;
    }, 1000);
  };
  return (
    <div className="grow">
      <VForm state={item} manager={manager}>
        <button className="btn-primary" type="button" onClick={perform_create}>
          Add
        </button>
      </VForm>
    </div>
  );
};

export const VEditItemForm = forwardRef<
  HTMLDivElement,
  {
    item: Item;
    // item_manager: { update: (updated: Item) => void };
    close_handler: () => void;
  }
>((props, ref) => {
  const init_state = { ...props.item };
  const [state, set_state] = useState(init_state);
  useEffect(() => {
    set_state(props.item);
  }, [props.item]);

  const manager: ItemInputUpdater<ChangeEvent<HTMLInputElement>> = {
    update_name: (e) => {
      set_state({ ...state, name: e.target.value });
    },
    update_url: (e) => {
      set_state({ ...state, url: e.target.value });
    },
    update_username: (e) => {
      set_state({ ...state, username: e.target.value });
    },
    update_password: (e) => {
      set_state({ ...state, password: e.target.value });
    },
  };

  const perform_save = async (e: MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.disabled = true;
    await invoke("update_item", { ...state });
    e.currentTarget.disabled = false;
    props.close_handler();
  };

  return (
    <div ref={ref} className="hide grow">
      <VForm state={state} manager={manager}>
        <div className="flex">
          <button className="btn-primary" type="button" onClick={perform_save}>
            Save
          </button>
          <button
            className="btn-primary"
            type="button"
            onClick={props.close_handler}
          >
            Cancel
          </button>
        </div>
      </VForm>
    </div>
  );
});
