"use client";

import {
  Button,
  Description,
  FieldError,
  Input,
  Label,
  TextField,
} from "@heroui/react";

export type AdminData = {
  name: string;
  email: string;
  password: string;
  projectName: string;
};

type Props = {
  value: AdminData;
  onChange: (v: AdminData) => void;
  onNext: () => void;
};

export function AdminStep({ value, onChange, onNext }: Props) {
  const valid =
    value.name.trim().length > 0 &&
    value.email.includes("@") &&
    value.password.length >= 8 &&
    value.projectName.trim().length > 0;

  return (
    <div className="flex flex-col gap-5">
      <TextField isRequired>
        <Label>Name</Label>
        <Input
          value={value.name}
          onChange={(event) => onChange({ ...value, name: event.target.value })}
        />
        <FieldError />
      </TextField>
      <TextField type="email" isRequired>
        <Label>Email</Label>
        <Input
          autoComplete="email"
          value={value.email}
          onChange={(event) => onChange({ ...value, email: event.target.value })}
        />
        <FieldError />
      </TextField>
      <TextField type="password" isRequired>
        <Label>Password</Label>
        <Input
          autoComplete="new-password"
          value={value.password}
          onChange={(event) =>
            onChange({ ...value, password: event.target.value })
          }
        />
        <Description>Minimum 8 characters</Description>
        <FieldError />
      </TextField>
      <TextField isRequired>
        <Label>Default project name</Label>
        <Input
          value={value.projectName}
          onChange={(event) =>
            onChange({ ...value, projectName: event.target.value })
          }
        />
        <FieldError />
      </TextField>

      <div className="flex justify-end pt-1">
        <Button variant="primary" onPress={onNext} isDisabled={!valid}>
          Continue
        </Button>
      </div>
    </div>
  );
}
