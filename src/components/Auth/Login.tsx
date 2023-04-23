import { MouseEvent, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useRouter } from "next/router";

// TODO: duplicate with Register => extract
export function Login() {
  const [password, set_password] = useState("");
  const router = useRouter();

  const login = (password: string) => {
    invoke("login", { password: password })
      .then((e) => {
        console.log(e);
        if (e) {
          router.replace({ pathname: "/vault" });
        }
      })
      .catch(console.error);
  };

  const perform_login = (e: MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    if (password.length < 1) {
      return;
    }

    e.currentTarget.disabled = true;
    login(password);
    e.currentTarget.disabled = false;
  };

  return (
    <>
      <form>
        <div className="mb-6">
          <label
            htmlFor="password"
            className="mb-2 block text-sm font-medium text-gray-900 dark:text-white"
          >
            Your password
          </label>
          <input
            type="password"
            id="password"
            className="block w-full rounded-lg border border-gray-300 bg-gray-50 p-2.5 text-sm text-gray-900 focus:border-blue-500 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400 dark:focus:border-blue-500 dark:focus:ring-blue-500"
            value={password}
            onChange={(e) => set_password(e.target.value)}
          />
        </div>

        <button
          onClick={perform_login}
          type="submit"
          className="w-full rounded-lg bg-blue-700 px-5 py-2.5 text-center text-sm font-medium text-white hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 sm:w-auto"
        >
          Submit
        </button>
      </form>
    </>
  );
}
