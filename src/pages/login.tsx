import Link from "next/link";
import { ChangeEvent } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";

export default function Login() {
  const [input_name, set_input_name] = useState("");
  const [display_output, set_display_output] = useState("");

  const input_name_handler = (e: ChangeEvent<HTMLInputElement>) => {
    set_input_name(e.target.value);
  };

  const perform_action = () => {
    invoke("greet", { name: input_name })
      .then((e) => set_display_output(e as string))
      .catch(console.error);
  };

  return (
    <>
      <div>
        <button>
          <Link href="/">Main</Link>
        </button>
        <h1 className=" text-3xl font-bold underline">Hello world!</h1>
        <div>
          <p>
            Text <span>{display_output}</span>
          </p>
        </div>
        <form>
          <label htmlFor="name" className="form-label">
            Name
          </label>
          <input
            type="text"
            id="name"
            placeholder="Name"
            name="name"
            value={input_name}
            onChange={input_name_handler}
          />
          <button type="button" onClick={perform_action}>
            Send
          </button>
        </form>
      </div>
    </>
  );
}
