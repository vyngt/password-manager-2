import { ChangeEvent } from "react";

export const Input = ({
  label,
  name,
  type,
  state,
  handler,
}: {
  label: string;
  name: string;
  type: string;
  state: string;
  handler: (e: ChangeEvent<HTMLInputElement>) => void;
}) => {
  return (
    <div className="mb-6">
      <label htmlFor={name} className="vault-label">
        {label}
      </label>
      <input
        className="vault-input"
        type={type}
        id={name}
        name={name}
        value={state}
        onChange={handler}
      />
    </div>
  );
};

export const TextInput = ({
  label,
  name,
  state,
  handler,
}: {
  label: string;
  name: string;
  state: string;
  handler: (e: ChangeEvent<HTMLInputElement>) => void;
}) => {
  return (
    <Input
      label={label}
      name={name}
      type="text"
      state={state}
      handler={handler}
    />
  );
};

export const PasswordInput = ({
  label,
  name,
  state,
  handler,
}: {
  label: string;
  name: string;
  state: string;
  handler: (e: ChangeEvent<HTMLInputElement>) => void;
}) => {
  return (
    <Input
      label={label}
      name={name}
      type="password"
      state={state}
      handler={handler}
    />
  );
};
