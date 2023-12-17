"use client";
import Base, { IBase } from "./Base";

const BaseValue: IBase = {
  title: "Login",
  button: "Login",
  handler: "login",
  error_msg: "Login Failed",
};

export default function Login() {
  return <Base base={BaseValue} />;
}
