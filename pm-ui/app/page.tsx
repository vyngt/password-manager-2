"use client";

import { invoke } from "@tauri-apps/api/tauri";
import { Input, IconButton } from "@/components/MaterialTailwind";
import { faArrowRight } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useRef, KeyboardEvent, useEffect } from "react";
import { useRouter } from "next/navigation";

export default function EnterMasterPassword() {
  const ref = useRef<HTMLInputElement>(null);
  const router = useRouter();

  const callServer = async (password: string) => {
    try {
      const ok = await invoke("perform_auth", { password: password });
      if (ok) {
        router.replace("/main");
      } else {
        // Simply ignore, nothing happen!
      }
    } catch (e) {
      // Simply ignore, nothing happen!
    }
  };

  const handleInput = () => {
    if (ref && ref.current) {
      const input = ref.current.querySelector("input");
      if (input) {
        const output = input.value;
        input.value = "";
        return output;
      }
    }

    return "";
  };

  const performAuth = async () => {
    const input = handleInput();
    if (input.length < 1) {
      return;
    }

    await callServer(input);
  };

  const handleKeypress = async (ev: KeyboardEvent<HTMLInputElement>) => {
    if (ref && ref.current && ev.key == "Enter") {
      ev.preventDefault();
      await performAuth();
    }
  };

  useEffect(() => {
    ref.current?.querySelector("input")?.focus();
  }, []);

  return (
    <div className="flex h-full w-full flex-col justify-center">
      <div className="flex w-full justify-center">
        <div className="relative flex w-full max-w-[24rem]">
          <Input
            ref={ref}
            crossOrigin={""}
            type="password"
            placeholder="Master Password"
            className="!border-primary/50 text-primary placeholder:text-primary/60 focus:!border-primary"
            onKeyDownCapture={handleKeypress}
            labelProps={{
              className: "before:content-none after:content-none",
            }}
            containerProps={{
              className: "min-w-0",
            }}
          />
          <IconButton
            onClick={performAuth}
            size="sm"
            variant="text"
            className="!absolute right-1 top-1 rounded bg-primary/40 text-foreground hover:bg-primary/60 active:bg-primary/80"
          >
            <FontAwesomeIcon icon={faArrowRight} />
          </IconButton>
        </div>
      </div>
    </div>
  );
}
