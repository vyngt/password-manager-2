"use client";
import Base, { IBase } from "./Base";

const BaseValue: IBase = {
  title: "Setup",
  button: "Set Master Password",
  handler: "register",
  error_msg: "Something went wrong!!!",
};

export default function Register() {
  return <Base base={BaseValue} />;
}
