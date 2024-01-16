"use client";

import { FormWrapper } from "@/components/UI/FormView";
import type { FormModel } from "@/components/UI/FormView";
import { ColorInput, Input } from "@/components/UI/Input";
import { hex2rgb, rgb2hex } from "@/lib/utils";

interface ColorScheme extends FormModel {
  name: string;
  primary: string;
  secondary: string;
  warning: string;
  success: string;
  danger: string;
  foreground: string;
  background: string;
}

const Wrapper = FormWrapper<ColorScheme>({
  initData: {
    id: 0,
    name: "Name",
    primary: "#ffffff",
    secondary: "#ffffff",
    warning: "#ffffff",
    success: "#ffffff",
    danger: "#ffffff",
    foreground: "#ffffff",
    background: "#ffffff",
  },
  crud: {
    deleteId: "delete_color_scheme",
    saveId: "update_color_scheme",
    getId: "get_color_scheme",
    createId: "create_color_scheme",
  },
  path: "/main/color-scheme/form",
});

export default function Page() {
  const Form = Wrapper(({ state, dispatch }) => {
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
        <div className="flex flex-wrap items-center justify-center gap-4">
          <ColorInput
            name="primary"
            value={rgb2hex(state.primary)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: { field: "primary", value: hex2rgb(ev.target.value) },
              })
            }
          />
          <ColorInput
            name="secondary"
            value={rgb2hex(state.secondary)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: {
                  field: "secondary",
                  value: hex2rgb(ev.target.value),
                },
              })
            }
          />
          <ColorInput
            name="success"
            value={rgb2hex(state.success)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: { field: "success", value: hex2rgb(ev.target.value) },
              })
            }
          />
          <ColorInput
            name="warning"
            value={rgb2hex(state.warning)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: { field: "warning", value: hex2rgb(ev.target.value) },
              })
            }
          />
          <ColorInput
            name="danger"
            value={rgb2hex(state.danger)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: { field: "danger", value: hex2rgb(ev.target.value) },
              })
            }
          />
          <ColorInput
            name="foreground"
            value={rgb2hex(state.foreground)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: {
                  field: "foreground",
                  value: hex2rgb(ev.target.value),
                },
              })
            }
          />
          <ColorInput
            name="background"
            value={rgb2hex(state.background)}
            onChange={(ev) =>
              dispatch({
                type: "patch",
                payload: {
                  field: "background",
                  value: hex2rgb(ev.target.value),
                },
              })
            }
          />
        </div>
      </>
    );
  });

  return <Form />;
}
