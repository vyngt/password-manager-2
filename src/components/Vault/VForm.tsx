import { ChangeEvent, MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";
import { NewItem } from "@/models";

export const VForm = () => {
  const [item, set_item] = useState<NewItem>({
    name: "",
    url: "",
    username: "",
    password: "",
  });

  const update_name = (e: ChangeEvent<HTMLInputElement>) => {
    set_item({ ...item, name: e.target.value });
  };
  const update_url = (e: ChangeEvent<HTMLInputElement>) => {
    set_item({ ...item, url: e.target.value });
  };
  const update_username = (e: ChangeEvent<HTMLInputElement>) => {
    set_item({ ...item, username: e.target.value });
  };
  const update_password = (e: ChangeEvent<HTMLInputElement>) => {
    set_item({ ...item, password: e.target.value });
  };

  const reset_value = () => {
    set_item({
      name: "",
      url: "",
      username: "",
      password: "",
    });
  };

  const perform_create = (e: MouseEvent<HTMLButtonElement>) => {
    const button = e.currentTarget;
    button.disabled = true;

    invoke("create_item", { ...item })
      .then(() => {
        console.log("successful");
      })
      .catch(console.error)
      .finally(() => {
        reset_value();
        setTimeout(() => {
          button.disabled = false;
        }, 1000);
      });
  };

  return (
    <div>
      <form>
        <div className="mb-6">
          <label htmlFor="name" className="vault-label">
            Name
          </label>
          <input
            className="vault-input"
            type="text"
            id="name"
            placeholder="Name"
            name="name"
            value={item.name}
            onChange={update_name}
          />
        </div>
        <div className="mb-6">
          <label htmlFor="url" className="vault-label">
            URL
          </label>
          <input
            className="vault-input"
            type="text"
            id="url"
            placeholder="url"
            name="url"
            value={item.url}
            onChange={update_url}
          />
        </div>
        <div className="mb-6">
          <label htmlFor="username" className="vault-label">
            username
          </label>
          <input
            className="vault-input"
            type="text"
            id="username"
            placeholder="username"
            name="username"
            value={item.username}
            onChange={update_username}
          />
        </div>
        <div className="mb-6">
          <label htmlFor="password" className="vault-label">
            password
          </label>
          <input
            className="vault-input"
            type="password"
            id="password"
            placeholder="password"
            name="password"
            value={item.password}
            onChange={update_password}
          />
        </div>
        <button className="btn-primary" type="button" onClick={perform_create}>
          Add
        </button>
      </form>
    </div>
  );
};
