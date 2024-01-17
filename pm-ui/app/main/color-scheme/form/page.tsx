"use client";

import { FormWrapper } from "@/components/UI/FormView";
import type { FormModel } from "@/components/UI/FormView";
import type { ColorScheme } from "@/components/ColorScheme/define";
import { ColorInput, Input } from "@/components/UI/Input";
import { hex2rgb, rgb2hex } from "@/lib/utils";

interface FormColorScheme extends FormModel, ColorScheme {}

const Wrapper = FormWrapper<FormColorScheme>({
  initData: {
    id: 0,
    name: "New Template",
    primary: "244 31 198",
    secondary: "3 169 244",
    warning: "224 193 36",
    success: "30 200 135",
    danger: "244 37 99",
    foreground: "255 194 194",
    background: "97 25 74",
  },
  crud: {
    deleteId: "delete_color_scheme",
    saveId: "update_color_scheme",
    getId: "get_color_scheme",
    createId: "create_color_scheme",
  },
  path: "/main/color-scheme/form",
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
