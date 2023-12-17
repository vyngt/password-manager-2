import { Button } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate, faPlus } from "@fortawesome/free-solid-svg-icons";
import { useManager } from "./Context";
import { ColorSchemeCreateForm } from "./ColorSchemeForm";

export const ColorSchemeManagerHeader = () => {
  const context = useManager();

  return (
    <div className="flex gap-3">
      <ColorSchemeCreateForm />
      <Button
        size="sm"
        variant="outlined"
        onClick={context.reload}
        className="border-pm-primary text-pm-primary"
      >
        <FontAwesomeIcon icon={faArrowsRotate} />
      </Button>
    </div>
  );
};
