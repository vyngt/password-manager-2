import { ChangeEvent, MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useState, forwardRef } from "react";
import { NewItem, Item } from "@/models";
import { TextInput, PasswordInput } from "./Input";

export const VForm = () => {
  const [item, set_item] = useState<NewItem>({
    name: "",
    url: "",
    username: "",
    password: "",
  });

  const operator = {
    update_name: (e: ChangeEvent<HTMLInputElement>) => {
      set_item({ ...item, name: e.target.value });
    },
    update_url: (e: ChangeEvent<HTMLInputElement>) => {
      set_item({ ...item, url: e.target.value });
    },
    update_username: (e: ChangeEvent<HTMLInputElement>) => {
      set_item({ ...item, username: e.target.value });
    },
    update_password: (e: ChangeEvent<HTMLInputElement>) => {
      set_item({ ...item, password: e.target.value });
    },
    reset_value: () => {
      set_item({
        name: "",
        url: "",
        username: "",
        password: "",
      });
    },
    perform_create: async (e: MouseEvent<HTMLButtonElement>) => {
      const button = e.currentTarget;
      button.disabled = true;
      const result = await invoke("create_item", { ...item });
      console.log("After submit", result);
      operator.reset_value();
      setTimeout(() => {
        button.disabled = false;
      }, 1000);
    },
  };

  return (
    <div className="grow">
      <form>
        <TextInput
          label={"Name"}
          name={"name"}
          state={item.name}
          handler={operator.update_name}
        />
        <TextInput
          label={"URL"}
          name={"url"}
          state={item.url}
          handler={operator.update_url}
        />
        <TextInput
          label={"Username"}
          name={"username"}
          state={item.username}
          handler={operator.update_username}
        />
        <PasswordInput
          label={"Password"}
          name={"password"}
          state={item.password}
          handler={operator.update_password}
        />
        <button
          className="btn-primary"
          type="button"
          onClick={operator.perform_create}
        >
          Add
        </button>
      </form>
    </div>
  );
};

export const VCreateForm = () => {
  return <></>;
};

export const VEditItemForm = forwardRef<
  HTMLDivElement,
  { item: Item; close_handler: () => void }
>((props, ref) => {
  return (
    <div ref={ref}>
      <div>Hello World</div>
      <button onClick={props.close_handler}>Me may</button>
    </div>
  );
});
