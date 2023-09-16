import { Dialog, Button, Typography } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { useState, useRef } from "react";
import {
  IBaseColorScheme,
  IColorScheme,
  IBaseManager,
  BaseColorScheme,
} from "@/components/Theme/types";

type ColorInputRef = {
  [P in keyof BaseColorScheme]: HTMLInputElement | null;
};

export default function ColorSchemeForm<T extends IBaseColorScheme>({
  open,
  toggle,
  data,
  action,
  name_action,
}: {
  open: boolean;
  toggle: () => void;
  data: T;
  action: (data: T) => void;
  name_action: string;
}) {
  const colorRef = useRef<ColorInputRef>({
    primary: null,
    secondary: null,
    success: null,
    danger: null,
    warning: null,
    foreground: null,
    background: null,
    selection: null,
  });

  const get_input_ref = () => {
    const input: BaseColorScheme = {
      primary: "",
      secondary: "",
      success: "",
      danger: "",
      warning: "",
      foreground: "",
      background: "",
      selection: "",
    };

    if (colorRef && colorRef.current) {
      const loop = Object.keys(colorRef.current) as Array<
        keyof BaseColorScheme
      >;
      for (const key of loop) {
        const element = colorRef.current[key];
        if (element) {
          input[key] = element.value;
        }
      }
    }

    const output = {
      ...data,
      ...input,
    };

    console.log(data, output);

    return output;
  };

  const handler_action = () => {
    console.log("Perform Action", get_input_ref());
    toggle();
  };

  return (
    <Dialog size="md" open={open} handler={() => {}} className="shadow-none">
      <div className="mx-auto flex w-full flex-col rounded-lg border border-pm-foreground bg-pm-background p-6">
        <div className="mb-4 flex justify-center">
          <Typography variant="h4" className="!text-pm-primary">
            {data.name}
          </Typography>
        </div>
        <div className="mb-3 flex justify-between">
          <div className="w-[49%]">
            <div className="relative h-10 w-full min-w-[200px]">
              <input
                className="pm-input peer text-pm-foreground"
                placeholder=" "
              />
              <label className="before:content[' '] after:content[' '] pm-input-label">
                Name
              </label>
            </div>
            <div className="mt-5 grid grid-cols-3 gap-2">
              {(Object.keys(data) as Array<keyof IColorScheme>).map(
                (key) =>
                  key != "name" &&
                  key != "id" && (
                    <div
                      className="relative flex flex-col justify-center gap-1"
                      key={`${data.name}-${key}`}
                    >
                      <label className="pm-input-color-container">
                        <input
                          type="color"
                          ref={(element) => (colorRef.current[key] = element)}
                        />
                      </label>
                      <Typography className="capitalize text-pm-primary">
                        {key}
                      </Typography>
                    </div>
                  ),
              )}
            </div>
          </div>
          <div className="w-[49%]">
            <div className="h-[200px] w-full">
              <div>Hello World</div>
            </div>
          </div>
        </div>
        <div className="flex justify-center gap-3">
          <Button onClick={handler_action}>{name_action}</Button>
          <Button onClick={toggle}>Trigger</Button>
        </div>
      </div>
    </Dialog>
  );
}

export const ColorSchemeUpdateForm = () => {
  const [open, setOpen] = useState(false);
  const toggle = () => setOpen((cur) => !cur);

  const [data, setData] = useState<IColorScheme>({
    id: 0,
    name: "Default",
    primary: "#000000",
    secondary: "#000000",
    success: "#000000",
    danger: "#000000",
    warning: "#000000",
    foreground: "#ffffff",
    background: "#000000",
    selection: "#000000",
  });

  return (
    <ColorSchemeForm
      toggle={toggle}
      action={(data) => {
        console.log("Updated", data);
      }}
      name_action="Update"
      data={data}
      open={open}
    />
  );
};

export const ColorSchemeCreateForm = () => {
  const [open, setOpen] = useState(false);
  const toggle = () => setOpen((cur) => !cur);

  const [data, setData] = useState<IBaseColorScheme>({
    name: "New Scheme",
    primary: "blue",
    secondary: "gray",
    success: "green",
    danger: "red",
    warning: "yellow",
    foreground: "#ffffff",
    background: "#000000",
    selection: "white",
  });

  return (
    <>
      <Button
        size="sm"
        onClick={toggle}
        className="bg-pm-primary text-pm-foreground"
      >
        <FontAwesomeIcon icon={faPlus} />
      </Button>
      <ColorSchemeForm
        toggle={toggle}
        action={(data) => {
          console.log("Create", data);
        }}
        name_action="Create"
        data={data}
        open={open}
      />
    </>
  );
};
