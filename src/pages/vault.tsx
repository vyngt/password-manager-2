import Link from "next/link";
import { ChangeEvent, MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";
import { Inter } from "next/font/google";

const inter = Inter({ subsets: ["latin"] });

interface NewItem {
  name: string;
  url: string;
  username: string;
  password: string;
}

interface Item extends NewItem {
  id: number;
}

export default function Vault() {
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

  const perform_fetch_items = () => {
    invoke("fetch_all_item")
      .then((e) => console.log(e))
      .catch(console.error);
  };

  return (
    <>
      <main>
        <div>
          <form>
            <div>
              <label htmlFor="name" className="form-label">
                Name
              </label>
              <input
                type="text"
                id="name"
                placeholder="Name"
                name="name"
                value={item.name}
                onChange={update_name}
              />
            </div>
            <div>
              <label htmlFor="url" className="form-label">
                URL
              </label>
              <input
                type="text"
                id="url"
                placeholder="url"
                name="url"
                value={item.url}
                onChange={update_url}
              />
            </div>
            <div>
              <label htmlFor="username" className="form-label">
                username
              </label>
              <input
                type="text"
                id="username"
                placeholder="username"
                name="username"
                value={item.username}
                onChange={update_username}
              />
            </div>
            <div>
              <label htmlFor="password" className="form-label">
                password
              </label>
              <input
                type="text"
                id="password"
                placeholder="password"
                name="password"
                value={item.password}
                onChange={update_password}
              />
            </div>
            <button type="button" onClick={perform_create}>
              Submit
            </button>
          </form>

          <button type="button" onClick={perform_fetch_items}>
            Fetch
          </button>
        </div>
      </main>
    </>
  );
}
