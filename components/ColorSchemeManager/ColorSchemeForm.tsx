import { Dialog, Button } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { useState } from "react";

export default function ColorSchemeForm({
  open,
  children,
}: {
  open: boolean;
  children?: React.ReactNode;
}) {
  return (
    <Dialog size="xs" open={open} handler={() => {}} className="shadow-none">
      <div className="mx-auto flex w-full flex-col rounded-lg border border-pm-foreground bg-pm-background p-6"></div>
    </Dialog>
  );
}

export const ColorSchemeUpdateForm = () => {
  const [open, setOpen] = useState(false);
  return <ColorSchemeForm open={open} />;
};

export const ColorSchemeCreateForm = () => {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button
        size="sm"
        onClick={() => {}}
        className="bg-pm-primary text-pm-foreground"
      >
        <FontAwesomeIcon icon={faPlus} />
      </Button>
      <ColorSchemeForm open={open}></ColorSchemeForm>
    </>
  );
};
