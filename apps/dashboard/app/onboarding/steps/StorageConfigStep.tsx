"use client";

import {
  Button,
  FieldError,
  Input,
  Label,
  TextArea,
  TextField,
} from "@heroui/react";
import type { StorageType } from "./StorageTypeStep";

export type StorageData = {
  host?: string;
  port?: number;
  username?: string;
  password?: string;
  private_key?: string;
  base_path?: string;
  public_base_url?: string;
};

type Props = {
  type: StorageType;
  value: StorageData;
  onChange: (v: StorageData) => void;
  onBack: () => void;
  onNext: () => void;
};

export function StorageConfigStep({
  type,
  value,
  onChange,
  onBack,
  onNext,
}: Props) {
  const set = (patch: Partial<StorageData>) => onChange({ ...value, ...patch });

  const valid = (() => {
    if (!value.base_path || !value.public_base_url) return false;
    if (type === "local") return true;
    if (!value.host || !value.username) return false;
    if (type === "ftp") return !!value.password;
    return !!(value.password || value.private_key);
  })();

  return (
    <div className="flex flex-col gap-5">
      <p className="text-sm text-default-500">
        Configuring <span className="font-medium text-foreground">{type.toUpperCase()}</span> storage.
      </p>

      {type !== "local" && (
        <>
          <TextField isRequired>
            <Label>Host</Label>
            <Input
              value={value.host ?? ""}
              onChange={(event) => set({ host: event.target.value })}
            />
            <FieldError />
          </TextField>
          <TextField type="number">
            <Label>Port</Label>
            <Input
              value={value.port?.toString() ?? ""}
              onChange={(event) =>
                set({ port: Number(event.target.value) || undefined })
              }
            />
            <FieldError />
          </TextField>
          <TextField isRequired>
            <Label>Username</Label>
            <Input
              value={value.username ?? ""}
              onChange={(event) => set({ username: event.target.value })}
            />
            <FieldError />
          </TextField>
          <TextField type="password">
            <Label>
              {type === "sftp" ? "Password (or use private key)" : "Password"}
            </Label>
            <Input
              value={value.password ?? ""}
              onChange={(event) => set({ password: event.target.value })}
            />
            <FieldError />
          </TextField>
        </>
      )}

      {type === "sftp" && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="private-key">Private key (optional)</Label>
          <TextArea
            id="private-key"
            rows={4}
            value={value.private_key ?? ""}
            onChange={(event) => set({ private_key: event.target.value })}
          />
        </div>
      )}

      <TextField isRequired>
        <Label>Base path</Label>
        <Input
          value={value.base_path ?? ""}
          onChange={(event) => set({ base_path: event.target.value })}
        />
        <FieldError />
      </TextField>
      <TextField isRequired>
        <Label>Public base URL</Label>
        <Input
          placeholder="https://example.com/uploads"
          value={value.public_base_url ?? ""}
          onChange={(event) => set({ public_base_url: event.target.value })}
        />
        <FieldError />
      </TextField>

      <div className="flex justify-end gap-2">
        <Button variant="tertiary" onPress={onBack}>
          Back
        </Button>
        <Button variant="primary" onPress={onNext} isDisabled={!valid}>
          Continue
        </Button>
      </div>
    </div>
  );
}
