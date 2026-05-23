"use client";

import {
  Button,
  Chip,
  Input,
  Label,
  TextField,
  useOverlayState,
} from "@heroui/react";
import { Boxes, CalendarDays, Fingerprint, Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { ApiError, type Project } from "../../../lib/api";
import {
  Alert,
  EmptyBlock,
  FormModal,
  LoadingBlock,
  ModalActions,
  PageHeader,
} from "../../../components/PageUI";
import {
  useCreateProject,
  useDeleteProject,
  useProjects,
  useUpdateProject,
} from "../../../lib/queries";

export function ProjectsPage() {
  const projects = useProjects();
  const create = useCreateProject();
  const update = useUpdateProject();
  const remove = useDeleteProject();
  const createModal = useOverlayState();
  const editModal = useOverlayState();
  const [editing, setEditing] = useState<Project | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function onDelete(id: string) {
    if (
      !confirm(
        "Delete this project and its related storage, presets, and API keys?",
      )
    )
      return;
    setError(null);
    try {
      await remove.mutateAsync(id);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Delete failed.");
    }
  }

  function openEdit(project: Project) {
    setEditing(project);
    editModal.open();
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={Boxes}
        title="Projects"
        description="Create isolated upload namespaces for your apps."
        action={
          <Button variant="primary" onPress={createModal.open}>
            <Plus className="h-4 w-4" /> New project
          </Button>
        }
      />

      {error && <Alert message={error} />}

      {projects.isPending ? (
        <LoadingBlock />
      ) : !projects.data?.length ? (
        <EmptyBlock
          icon={Boxes}
          title="No projects yet"
          description="Create your first project to start organizing uploads."
          action={
            <Button variant="primary" onPress={createModal.open}>
              <Plus className="h-4 w-4" /> New project
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {projects.data.map((project) => (
            <article
              key={project.id}
              className="group relative overflow-hidden rounded-3xl border border-default-200 bg-background p-5 shadow-sm transition hover:-translate-y-0.5 hover:border-accent/40 hover:shadow-md"
            >
              <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-accent to-primary opacity-80" />
              <div className="flex items-start justify-between gap-3">
                <div className="flex h-11 w-11 items-center justify-center rounded-2xl bg-accent/10 text-accent ring-1 ring-accent/15">
                  <Boxes className="h-5 w-5" />
                </div>
                <Chip size="sm" variant="soft" color="default">
                  {project.slug}
                </Chip>
              </div>
              <div className="min-w-0">
                <h2 className="truncate text-base font-semibold">
                  {project.name}
                </h2>
                <p className="mt-1 truncate font-mono text-xs text-default-400">
                  {project.id}
                </p>
              </div>
              <div className="grid gap-3 text-xs text-default-500">
                <div className="flex items-center gap-2 rounded-2xl border border-default-100 bg-default-50 px-3 py-2">
                  <Fingerprint className="h-3.5 w-3.5" />
                  <span className="truncate font-mono">{project.slug}</span>
                </div>
                <div className="flex items-center gap-2 rounded-2xl border border-default-100 bg-default-50 px-3 py-2">
                  <CalendarDays className="h-3.5 w-3.5" />
                  <span>Created {new Date(project.created_at).toLocaleDateString()}</span>
                </div>
              </div>
              <div className="flex gap-2 border-t border-default-100 pt-3">
                <Button
                  size="sm"
                  variant="tertiary"
                  onPress={() => openEdit(project)}
                  className="flex-1"
                >
                  <Pencil className="h-3.5 w-3.5" /> Edit
                </Button>
                <Button
                  size="sm"
                  variant="danger-soft"
                  onPress={() => onDelete(project.id)}
                  isPending={remove.isPending}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </article>
          ))}
        </div>
      )}

      <FormModal
        state={createModal}
        title="New project"
        description="Create a new upload namespace."
      >
        <ProjectForm
          onCancel={createModal.close}
          onSave={async (value) => {
            await create.mutateAsync(value);
            createModal.close();
          }}
          saving={create.isPending}
        />
      </FormModal>

      <FormModal
        state={editModal}
        title="Edit project"
        description="Rename your project or change its slug."
      >
        {editing && (
          <ProjectForm
            project={editing}
            onCancel={editModal.close}
            onSave={async (value) => {
              await update.mutateAsync({ id: editing.id, body: value });
              editModal.close();
            }}
            saving={update.isPending}
          />
        )}
      </FormModal>
    </div>
  );
}

function ProjectForm({
  project,
  onCancel,
  onSave,
  saving,
}: {
  project?: Project;
  onCancel: () => void;
  onSave: (value: { name: string; slug?: string }) => Promise<void>;
  saving: boolean;
}) {
  const [name, setName] = useState(project?.name ?? "");
  const [slug, setSlug] = useState(project?.slug ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setName(project?.name ?? "");
    setSlug(project?.slug ?? "");
  }, [project]);

  async function submit() {
    setError(null);
    try {
      await onSave({ name, slug: slug || undefined });
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Save failed.");
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <TextField isRequired>
        <Label>Name</Label>
        <Input value={name} onChange={(e) => setName(e.target.value)} />
      </TextField>
      <TextField>
        <Label>Slug</Label>
        <Input
          value={slug}
          onChange={(e) => setSlug(e.target.value)}
          placeholder="my-app"
        />
      </TextField>
      {error && <Alert message={error} />}
      <ModalActions>
        <Button variant="tertiary" onPress={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onPress={submit}
          isPending={saving}
          isDisabled={!name.trim()}
        >
          {project ? "Save changes" : "Create project"}
        </Button>
      </ModalActions>
    </div>
  );
}
