import { Dialog, Button, Typography } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { useState, useRef, useEffect } from "react";
import { IBaseColorScheme, IColorScheme } from "@/components/Theme/types";
import { useManager } from "./Context";

type ColorSchemeInputRef = {
  [P in keyof IBaseColorScheme]: HTMLInputElement | null;
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
  const colorSchemeRef = useRef<ColorSchemeInputRef>({
    name: null,
    primary: null,
    secondary: null,
    success: null,
    danger: null,
    warning: null,
    foreground: null,
    background: null,
    selection: null,
  });

  const getColorSchemeData = () => {
    const input: IBaseColorScheme = {
      name: "",
      primary: "",
      secondary: "",
      success: "",
      danger: "",
      warning: "",
      foreground: "",
      background: "",
      selection: "",
    };

    if (colorSchemeRef && colorSchemeRef.current) {
      const loop = Object.keys(colorSchemeRef.current) as Array<
        keyof IBaseColorScheme
      >;
      for (const key of loop) {
        const element = colorSchemeRef.current[key];
        if (element) {
          input[key] = element.value;
        }
      }
    }

    const output = {
      ...data,
      ...input,
    };

    return output;
  };

  const handler_action = () => {
    const colors = getColorSchemeData();
    action(colors);
  };

  useEffect(() => {
    if (colorSchemeRef && colorSchemeRef.current) {
      const loop = Object.keys(colorSchemeRef.current) as Array<
        keyof IBaseColorScheme
      >;
      for (const key of loop) {
        const element = colorSchemeRef.current[key];
        if (element) {
          element.value = data[key];
        }
      }
    }
  }, [data]);

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
                ref={(element) => (colorSchemeRef.current["name"] = element)}
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
                          ref={(element) =>
                            (colorSchemeRef.current[key] = element)
                          }
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
              <Typography variant="lead">Coming soon!</Typography>
            </div>
          </div>
        </div>
        <div className="flex justify-center gap-3">
          <Button
            onClick={handler_action}
            className="!bg-pm-primary !text-pm-foreground"
          >
            {name_action}
          </Button>
          <Button
            variant="outlined"
            className="!border-pm-primary !text-pm-foreground"
            onClick={toggle}
          >
            Close
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

export const ColorSchemeUpdateForm = ({
  data,
  open,
  toggle,
  action,
}: {
  data: IColorScheme;
  open: boolean;
  toggle: () => void;
  action: (scheme: IColorScheme) => void;
}) => {
  return (
    <ColorSchemeForm
      toggle={toggle}
      action={action}
      name_action="Update"
      data={data}
      open={open}
    />
  );
};

export const ColorSchemeCreateForm = () => {
  const context = useManager();
  const [open, setOpen] = useState(false);
  const toggle = () => setOpen((cur) => !cur);

  const data_sample: IBaseColorScheme = {
    name: "New Scheme",
    primary: "#800047",
    secondary: "#808080",
    success: "#008000",
    danger: "#ff0000",
    warning: "#ffff00",
    foreground: "#ffffff",
    background: "#000000",
    selection: "#ffffff",
  };

  const action = (data: IBaseColorScheme) => {
    setOpen((cur) => !cur);
    context.add(data);
  };

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
        action={action}
        name_action="Create"
        data={data_sample}
        open={open}
      />
    </>
  );
};
