"use client";

import { Button, Chip, Input, Label, TextArea, TextField, useOverlayState } from "@heroui/react";
import { FileType2, Folder, Gauge, Image, Pencil, Plus, Settings, Sparkles, Trash2, type LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import {
  Alert,
  EmptyBlock,
  FormModal,
  LoadingBlock,
  ModalActions,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import { ApiError, type UploadPreset } from "../../../lib/api";
import {
  useCreateUploadPreset,
  useDeleteUploadPreset,
  useProjects,
  useStorageConnections,
  useUpdateUploadPreset,
  useUploadPresets,
} from "../../../lib/queries";

type FormValue = {
  project_id: string;
  storage_connection_id: string;
  name: string;
  folder: string;
  allowed_mime_types: string;
  allowed_extensions: string;
  max_file_size: string;
  duplicate_strategy: UploadPreset["duplicate_strategy"];
  filename_strategy: string;
  transformations: string;
};

export function PresetsPage() {
  const projects = useProjects();
  const connections = useStorageConnections();
  const presets = useUploadPresets();
  const create = useCreateUploadPreset();
  const update = useUpdateUploadPreset();
  const remove = useDeleteUploadPreset();
  const createModal = useOverlayState();
  const editModal = useOverlayState();
  const [editing, setEditing] = useState<UploadPreset | null>(null);
  const [error, setError] = useState<string | null>(null);

  const defaultProjectId = projects.data?.[0]?.id ?? "";

  function openEdit(preset: UploadPreset) {
    setEditing(preset);
    editModal.open();
  }

  async function onDelete(id: string) {
    if (!confirm("Delete this upload preset?")) return;
    setError(null);
    try {
      await remove.mutateAsync(id);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Delete failed.");
    }
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={Settings}
        title="Upload presets"
        description="Shape every upload with reusable validation rules, folder routing, naming strategy, duplicate behavior, and transformation settings."
        action={
          <Button variant="primary" onPress={createModal.open} isDisabled={!defaultProjectId}>
            <Plus className="h-4 w-4" /> New preset
          </Button>
        }
      />

      {error && <Alert message={error} />}

      {presets.isPending ? (
        <LoadingBlock />
      ) : !presets.data?.length ? (
        <EmptyBlock
          icon={Settings}
          title="No upload presets"
          description="Create presets for avatars, documents, product images, or any other upload flow."
          action={
            <Button variant="primary" onPress={createModal.open} isDisabled={!defaultProjectId}>
              <Plus className="h-4 w-4" /> New preset
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {presets.data.map((preset) => (
            <PresetCard
              key={preset.id}
              preset={preset}
              deleting={remove.isPending}
              onEdit={() => openEdit(preset)}
              onDelete={() => onDelete(preset.id)}
            />
          ))}
        </div>
      )}

      <FormModal
        state={createModal}
        title="Create upload preset"
        description="Define the constraints and destination behavior for one upload use case."
        size="lg"
      >
        <PresetForm
          defaultProjectId={defaultProjectId}
          connections={connections.data ?? []}
          onCancel={createModal.close}
          onSave={async (value) => {
            await create.mutateAsync(toCreatePayload(value));
            createModal.close();
          }}
          saving={create.isPending}
        />
      </FormModal>

      <FormModal
        state={editModal}
        title="Edit upload preset"
        description="Adjust validation, storage, naming, and transformation rules."
        size="lg"
      >
        {editing && (
          <PresetForm
            preset={editing}
            defaultProjectId={editing.project_id}
            connections={connections.data ?? []}
            onCancel={editModal.close}
            onSave={async (value) => {
              await update.mutateAsync({ id: editing.id, body: toUpdatePayload(value) });
              editModal.close();
            }}
            saving={update.isPending}
          />
        )}
      </FormModal>
    </div>
  );
}

function PresetCard({ preset, deleting, onEdit, onDelete }: { preset: UploadPreset; deleting: boolean; onEdit: () => void; onDelete: () => void }) {
  return (
    <article className="relative overflow-hidden rounded-3xl border border-default-200 bg-background p-5 shadow-sm transition hover:-translate-y-0.5 hover:border-accent/40 hover:shadow-md">
      <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-primary to-accent opacity-80" />
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-start gap-3 min-w-0">
          <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-accent/10 text-accent">
            <Sparkles className="h-5 w-5" />
          </div>
          <div className="min-w-0">
            <h2 className="truncate text-base font-semibold">{preset.name}</h2>
            <p className="mt-1 truncate font-mono text-xs text-default-400">{preset.id}</p>
          </div>
        </div>
        <Chip size="sm" variant="soft" color="default">{preset.duplicate_strategy.replaceAll("_", " ")}</Chip>
      </div>

      <div className="mt-5 grid gap-3 sm:grid-cols-3">
        <Metric icon={Folder} label="Folder" value={preset.folder} />
        <Metric icon={Gauge} label="Max size" value={formatBytes(preset.max_file_size)} />
        <Metric icon={FileType2} label="Extensions" value={preset.allowed_extensions.length ? preset.allowed_extensions.join(", ") : "Any"} />
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <Chip size="sm" variant="soft" color="default">{preset.filename_strategy.replaceAll("_", " ")}</Chip>
        <Chip size="sm" variant="soft" color={preset.allowed_mime_types.length ? "success" : "default"}>
          {preset.allowed_mime_types.length ? `${preset.allowed_mime_types.length} MIME rules` : "Any MIME"}
        </Chip>
        <Chip size="sm" variant="soft" color={Object.keys(preset.transformations ?? {}).length ? "success" : "default"}>
          <Image className="h-3 w-3" /> {Object.keys(preset.transformations ?? {}).length ? "Transforms" : "No transforms"}
        </Chip>
      </div>

      <div className="mt-5 flex justify-end gap-2 border-t border-default-100 pt-4">
        <Button size="sm" variant="tertiary" onPress={onEdit}>
          <Pencil className="h-3.5 w-3.5" /> Edit
        </Button>
        <Button size="sm" variant="danger-soft" onPress={onDelete} isPending={deleting}>
          <Trash2 className="h-3.5 w-3.5" /> Delete
        </Button>
      </div>
    </article>
  );
}

function PresetForm({ preset, defaultProjectId, connections, onCancel, onSave, saving }: { preset?: UploadPreset; defaultProjectId: string; connections: { id: string; project_id: string; type: string; base_path: string }[]; onCancel: () => void; onSave: (value: FormValue) => Promise<void>; saving: boolean }) {
  const [value, setValue] = useState<FormValue>(() => initialValue(preset, defaultProjectId));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(initialValue(preset, defaultProjectId));
  }, [preset, defaultProjectId]);

  const set = (patch: Partial<FormValue>) => setValue((v) => ({ ...v, ...patch }));

  async function submit() {
    setError(null);
    try {
      JSON.parse(value.transformations);
      await onSave(value);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Transformations must be valid JSON.");
    }
  }

  return (
    <div className="flex flex-col gap-5">
      <div className="grid gap-3 sm:grid-cols-2">
        <TextField isRequired><Label>Name</Label><Input value={value.name} onChange={(e) => set({ name: e.target.value })} /></TextField>
        <TextField isRequired><Label>Folder</Label><Input value={value.folder} onChange={(e) => set({ folder: e.target.value })} /></TextField>
        <TextField><Label>Allowed MIME types</Label><Input value={value.allowed_mime_types} onChange={(e) => set({ allowed_mime_types: e.target.value })} placeholder="image/jpeg, image/png" /></TextField>
        <TextField><Label>Allowed extensions</Label><Input value={value.allowed_extensions} onChange={(e) => set({ allowed_extensions: e.target.value })} placeholder="jpg, png, webp" /></TextField>
        <TextField type="number" isRequired><Label>Max file size</Label><Input value={value.max_file_size} onChange={(e) => set({ max_file_size: e.target.value })} /></TextField>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium text-default-700">Storage connection</span>
          <NativeSelect value={value.storage_connection_id} onChange={(e) => set({ storage_connection_id: e.target.value })}>
            <option value="">None</option>
            {connections.filter((c) => c.project_id === value.project_id).map((c) => <option key={c.id} value={c.id}>{c.type} · {c.base_path}</option>)}
          </NativeSelect>
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium text-default-700">Duplicate strategy</span>
          <NativeSelect value={value.duplicate_strategy} onChange={(e) => set({ duplicate_strategy: e.target.value as FormValue["duplicate_strategy"] })}>
            <option value="return_existing">Return existing</option>
            <option value="upload_new_copy">Upload new copy</option>
            <option value="reject_duplicate">Reject duplicate</option>
          </NativeSelect>
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium text-default-700">Filename strategy</span>
          <NativeSelect value={value.filename_strategy} onChange={(e) => set({ filename_strategy: e.target.value })}>
            <option value="slug_random">Slug + random</option>
            <option value="uuid">UUID</option>
            <option value="hash">Hash</option>
            <option value="timestamp_random">Timestamp + random</option>
            <option value="random">Random</option>
            <option value="original_suffix">Original + suffix</option>
          </NativeSelect>
        </label>
      </div>

      <div className="flex flex-col gap-2">
        <Label>Transformations JSON</Label>
        <TextArea rows={6} value={value.transformations} onChange={(e) => set({ transformations: e.target.value })} />
      </div>

      {error && <Alert message={error} />}
      <ModalActions>
        <Button variant="tertiary" onPress={onCancel}>Cancel</Button>
        <Button variant="primary" onPress={submit} isPending={saving} isDisabled={!value.name.trim() || !value.folder.trim()}>
          {preset ? "Save changes" : "Create preset"}
        </Button>
      </ModalActions>
    </div>
  );
}

function initialValue(preset: UploadPreset | undefined, defaultProjectId: string): FormValue {
  return {
    project_id: preset?.project_id ?? defaultProjectId,
    storage_connection_id: preset?.storage_connection_id ?? "",
    name: preset?.name ?? "default",
    folder: preset?.folder ?? "uploads",
    allowed_mime_types: preset?.allowed_mime_types.join(", ") ?? "",
    allowed_extensions: preset?.allowed_extensions.join(", ") ?? "",
    max_file_size: String(preset?.max_file_size ?? 10485760),
    duplicate_strategy: preset?.duplicate_strategy ?? "return_existing",
    filename_strategy: preset?.filename_strategy ?? "slug_random",
    transformations: JSON.stringify(preset?.transformations ?? {}, null, 2),
  };
}

function Metric({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-default-100 bg-default-50 p-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-default-500"><Icon className="h-3.5 w-3.5" /> {label}</div>
      <p className="truncate text-sm text-default-700">{value}</p>
    </div>
  );
}

function toList(value: string) {
  return value.split(",").map((v) => v.trim()).filter(Boolean);
}

function toCreatePayload(value: FormValue) {
  return {
    project_id: value.project_id,
    storage_connection_id: value.storage_connection_id || undefined,
    name: value.name,
    folder: value.folder,
    allowed_mime_types: toList(value.allowed_mime_types),
    allowed_extensions: toList(value.allowed_extensions),
    max_file_size: Number(value.max_file_size),
    duplicate_strategy: value.duplicate_strategy,
    filename_strategy: value.filename_strategy,
    transformations: JSON.parse(value.transformations),
  };
}

function toUpdatePayload(value: FormValue) {
  const { project_id: _projectId, ...payload } = toCreatePayload(value);
  return payload;
}

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(bytes % (1024 * 1024) === 0 ? 0 : 1)} MB`;
}
