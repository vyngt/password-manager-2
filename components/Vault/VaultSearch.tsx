import { useState } from "react";
import { Input, Button } from "@material-tailwind/react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faMagnifyingGlass } from "@fortawesome/free-solid-svg-icons";

export function VaultSearch() {
  const [query, setQuery] = useState("");

  return (
    <div className="relative flex w-full max-w-[24rem]">
      <Input
        type="text"
        label="Keyword"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="pr-20"
        crossOrigin={""}
      />
      <Button
        size="sm"
        color={query ? "gray" : "blue-gray"}
        disabled={!query}
        className="!absolute right-1 top-1 rounded"
      >
        <FontAwesomeIcon icon={faMagnifyingGlass} />
      </Button>
    </div>
  );
}
