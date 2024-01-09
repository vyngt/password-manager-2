"use client";

import { useEffect } from "react";

export default function Page() {
  useEffect(() => {
    console.log("Worked");
  }, []);

  return <div>Create Form</div>;
}
