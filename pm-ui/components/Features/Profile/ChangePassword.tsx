"use client";
import { Input } from "@/components/UI";
import { Button } from "@/components/MaterialTailwind";
import { Section } from "./Section";

export function ChangePassword() {
  return (
    <Section name="Change Master Password">
      <div className="flex max-w-[400px] flex-col gap-3">
        <Input type="password" color="primary" label="New Password" />
        <Input type="password" color="primary" label="Confirm Password" />
        <div>
          <Button className="bg-primary/50 text-foreground">Update</Button>
        </div>
      </div>
    </Section>
  );
}
