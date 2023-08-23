import { Typography, Button } from "@material-tailwind/react";

export default function VaultPaginator() {
  return (
    <>
      <Typography variant="small" color="blue-gray" className="font-normal">
        Page 1 of 10
      </Typography>
      <div className="flex gap-2">
        <Button variant="outlined" size="sm">
          Previous
        </Button>
        <Button variant="outlined" size="sm">
          Next
        </Button>
      </div>
    </>
  );
}
