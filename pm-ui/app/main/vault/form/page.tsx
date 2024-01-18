"use client";

import { FormWrapper } from "@/components/UI/FormView";
import type { FormModel } from "@/components/UI/FormView";
import type { ItemInDB } from "@/components/Features/Vault/define";
import { Input } from "@/components/UI/Input";

interface FormItem extends FormModel, ItemInDB {}

const Wrapper = FormWrapper<FormItem>({
  initData: {
    id: 0,
    password: "",
    name: "",
    url: "",
    username: "",
  },
  crud: {
    deleteId: "delete_item",
    saveId: "update_item",
    getId: "get_item",
    createId: "create_item",
  },
  path: "/main/vault/form",
});

export default Wrapper(({ state, dispatch }) => {
  return (
    <>
      <Input
        color="primary"
        label="Name"
        value={state.name}
        onChange={(ev) =>
          dispatch({
            type: "patch",
            payload: { field: "name", value: ev.target.value },
          })
        }
      />
      <Input
        color="primary"
        label="URL"
        value={state.url}
        onChange={(ev) =>
          dispatch({
            type: "patch",
            payload: { field: "url", value: ev.target.value },
          })
        }
      />
      <Input
        color="primary"
        label="Username"
        value={state.username}
        onChange={(ev) =>
          dispatch({
            type: "patch",
            payload: { field: "username", value: ev.target.value },
          })
        }
      />
      <Input
        color="primary"
        label="Password"
        type="password"
        value={state.password}
        onChange={(ev) =>
          dispatch({
            type: "patch",
            payload: { field: "password", value: ev.target.value },
          })
        }
      />
    </>
  );
});
