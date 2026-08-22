"use client";

import {
  Button,
  FieldError,
  Input,
  Label,
  TextArea,
  TextField,
} from "@heroui/react";
import { useState } from "react";
import {
  ApiError,
  type CreateStorageConnectionRequest,
  type StorageConnection,
} from "../../../lib/api";
import {
  useCreateStorageConnection,
  useUpdateStorageConnection,
} from "../../../lib/queries";

export type StorageType = StorageConnection["type"];

export type ConnectionFormValue = {
  host?: string;
  port?: number;
  username?: string;
  password?: string;
  private_key?: string;
  bucket?: string;
  region?: string;
  endpoint?: string;
  access_key?: string;
  secret_key?: string;
  force_path_style?: boolean;
  base_path: string;
  public_base_url: string;
};

type CreateProps = {
  mode: "create";
  projectId: string;
  onCancel: () => void;
  onSaved: () => void;
};

type UpdateProps = {
  mode: "update";
  connectionId: string;
  connectionType: StorageType;
  value: ConnectionFormValue;
  onCancel: () => void;
  onSaved: () => void;
};

type Props = CreateProps | UpdateProps;

export function ConnectionForm(props: Props) {
  const create = useCreateStorageConnection();
  const update = useUpdateStorageConnection();
  const [error, setError] = useState<string | null>(null);

  const [type, setType] = useState<StorageType>(
    props.mode === "update" ? props.connectionType : "local",
  );
  const [value, setValue] = useState<ConnectionFormValue>(() =>
    props.mode === "update"
      ? props.value
      : { base_path: "", public_base_url: "" },
  );

  const set = (patch: Partial<ConnectionFormValue>) =>
    setValue((v) => ({ ...v, ...patch }));

  const validCommon = !!value.base_path && !!value.public_base_url;
  const validCreate = (() => {
    if (!validCommon) return false;
    if (type === "local") return true;
    if (type === "s3")
      return !!value.bucket && !!value.region && !!value.access_key && !!value.secret_key;
    if (!value.host || !value.username) return false;
    if (type === "ftp") return !!value.password;
    return !!(value.password || value.private_key);
  })();
  const valid = props.mode === "create" ? validCreate : validCommon;

  async function onSubmit() {
    setError(null);
    try {
      if (props.mode === "create") {
        const payload = buildCreatePayload(props.projectId, type, value);
        await create.mutateAsync(payload);
      } else {
        await update.mutateAsync({
          id: props.connectionId,
          body: {
            host: value.host || undefined,
            port: value.port,
            username: value.username || undefined,
            password: (type === "s3" ? value.secret_key : value.password)
              ? type === "s3"
                ? value.secret_key
                : value.password
              : undefined,
            private_key: value.private_key ? value.private_key : undefined,
            bucket: value.bucket || undefined,
            region: value.region || undefined,
            force_path_style: value.force_path_style,
            base_path: value.base_path,
            public_base_url: value.public_base_url,
          },
        });
      }
      props.onSaved();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Save failed.");
    }
  }

  const submitting = create.isPending || update.isPending;

  return (
    <div className="flex flex-col gap-4">
      {props.mode === "create" && (
        <fieldset className="flex flex-wrap gap-2">
          {(["local", "ftp", "sftp", "s3"] as StorageType[]).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => setType(t)}
              className={
                "rounded-lg border px-3 py-1.5 text-xs font-medium uppercase tracking-wide transition " +
                (type === t
                  ? "border-accent bg-accent/10 text-accent"
                  : "border-default-200 text-default-600 hover:bg-default-100")
              }
            >
              {t}
            </button>
          ))}
        </fieldset>
      )}

      {(type === "ftp" || type === "sftp") && (
        <div className="grid gap-3 sm:grid-cols-2">
          <TextField isRequired={props.mode === "create"}>
            <Label>Host</Label>
            <Input
              value={value.host ?? ""}
              onChange={(e) => set({ host: e.target.value })}
            />
            <FieldError />
          </TextField>
          <TextField type="number">
            <Label>Port</Label>
            <Input
              value={value.port?.toString() ?? ""}
              onChange={(e) =>
                set({ port: Number(e.target.value) || undefined })
              }
            />
            <FieldError />
          </TextField>
          <TextField isRequired={props.mode === "create"}>
            <Label>Username</Label>
            <Input
              value={value.username ?? ""}
              onChange={(e) => set({ username: e.target.value })}
            />
            <FieldError />
          </TextField>
          <TextField type="password">
            <Label>
              {props.mode === "update"
                ? "Password (leave blank to keep)"
                : type === "sftp"
                  ? "Password (or use private key)"
                  : "Password"}
            </Label>
            <Input
              value={value.password ?? ""}
              onChange={(e) => set({ password: e.target.value })}
            />
            <FieldError />
          </TextField>
        </div>
      )}

      {type === "s3" && (
        <div className="rounded-2xl border border-default-100 bg-default-50 p-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <TextField isRequired={props.mode === "create"}>
              <Label>Bucket</Label>
              <Input
                value={value.bucket ?? ""}
                onChange={(e) => set({ bucket: e.target.value })}
              />
              <FieldError />
            </TextField>
            <TextField isRequired={props.mode === "create"}>
              <Label>Region</Label>
              <Input
                placeholder="us-east-1"
                value={value.region ?? ""}
                onChange={(e) => set({ region: e.target.value })}
              />
              <FieldError />
            </TextField>
            <TextField>
              <Label>Endpoint (optional)</Label>
              <Input
                placeholder="https://s3.us-east-1.amazonaws.com"
                value={value.endpoint ?? ""}
                onChange={(e) => set({ endpoint: e.target.value, host: e.target.value })}
              />
              <FieldError />
            </TextField>
            <TextField isRequired={props.mode === "create"}>
              <Label>Access key</Label>
              <Input
                value={value.access_key ?? ""}
                onChange={(e) => set({ access_key: e.target.value, username: e.target.value })}
              />
              <FieldError />
            </TextField>
            <TextField type="password">
              <Label>
                {props.mode === "update"
                  ? "Secret key (leave blank to keep)"
                  : "Secret key"}
              </Label>
              <Input
                value={value.secret_key ?? ""}
                onChange={(e) => set({ secret_key: e.target.value })}
              />
              <FieldError />
            </TextField>
          </div>
          <label className="mt-3 flex items-center gap-2 text-sm text-default-700">
            <input
              type="checkbox"
              checked={value.force_path_style ?? false}
              onChange={(e) => set({ force_path_style: e.target.checked })}
            />
            Force path-style addressing
          </label>
          <p className="mt-1 text-xs text-default-400">
            Enable for providers that use path-style URLs, e.g. non-AWS S3-compatible services.
          </p>
        </div>
      )}

      {type === "sftp" && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="conn-private-key">
            Private key{props.mode === "update" ? " (leave blank to keep)" : " (optional)"}
          </Label>
          <TextArea
            id="conn-private-key"
            rows={4}
            value={value.private_key ?? ""}
            onChange={(e) => set({ private_key: e.target.value })}
          />
        </div>
      )}

      <div className="grid gap-3 sm:grid-cols-2">
        <TextField isRequired>
          <Label>{type === "s3" ? "Key prefix" : "Base path"}</Label>
          <Input
            value={value.base_path}
            onChange={(e) => set({ base_path: e.target.value })}
          />
          <FieldError />
        </TextField>
        <TextField isRequired>
          <Label>Public base URL</Label>
          <Input
            placeholder="https://example.com/uploads"
            value={value.public_base_url}
            onChange={(e) => set({ public_base_url: e.target.value })}
          />
          <FieldError />
        </TextField>
      </div>

      {error && (
        <div
          role="alert"
          className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
        >
          {error}
        </div>
      )}

      <div className="flex justify-end gap-2">
        <Button variant="tertiary" onPress={props.onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onPress={onSubmit}
          isPending={submitting}
          isDisabled={!valid}
        >
          {props.mode === "create" ? "Create connection" : "Save changes"}
        </Button>
      </div>
    </div>
  );
}

function buildCreatePayload(
  projectId: string,
  type: StorageType,
  v: ConnectionFormValue,
): CreateStorageConnectionRequest {
  if (type === "local") {
    return {
      project_id: projectId,
      type: "local",
      base_path: v.base_path,
      public_base_url: v.public_base_url,
    };
  }
  if (type === "ftp") {
    return {
      project_id: projectId,
      type: "ftp",
      host: v.host!,
      port: v.port,
      username: v.username!,
      password: v.password!,
      base_path: v.base_path,
      public_base_url: v.public_base_url,
    };
  }
  if (type === "sftp") {
    return {
      project_id: projectId,
      type: "sftp",
      host: v.host!,
      port: v.port,
      username: v.username!,
      password: v.password || undefined,
      private_key: v.private_key || undefined,
      base_path: v.base_path,
      public_base_url: v.public_base_url,
    };
  }
  return {
    project_id: projectId,
    type: "s3",
    bucket: v.bucket!,
    region: v.region!,
    endpoint: v.endpoint || undefined,
    access_key: v.access_key!,
    secret_key: v.secret_key!,
    force_path_style: v.force_path_style,
    base_path: v.base_path,
    public_base_url: v.public_base_url,
  };
}
