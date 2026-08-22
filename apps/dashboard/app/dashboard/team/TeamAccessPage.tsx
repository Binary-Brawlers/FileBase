"use client";

import {
  Button,
  Input,
  Label,
  TextField,
  useOverlayState,
} from "@heroui/react";
import {
  Check,
  Copy,
  Mail,
  ShieldCheck,
  Trash2,
  UserPlus,
  UsersRound,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  EmptyBlock,
  FormModal,
  LoadingBlock,
  ModalActions,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import {
  ApiError,
  type CreatedProjectInvitation,
  type ProjectRole,
} from "../../../lib/api";
import {
  useCreateProjectInvitation,
  useProjectInvitations,
  useProjectMembers,
  useProjects,
  useRemoveProjectMember,
  useRevokeProjectInvitation,
  useUpdateProjectMember,
} from "../../../lib/queries";

type AssignableRole = Exclude<ProjectRole, "owner">;

const ROLE_COPY: Record<ProjectRole, string> = {
  owner: "Full control, including project deletion.",
  admin: "Manages infrastructure, settings, and teammates.",
  editor: "Manages files, presets, and webhooks.",
  viewer: "Reads files, logs, analytics, and settings.",
};

export function TeamAccessPage() {
  const projects = useProjects();
  const [projectId, setProjectId] = useState("");
  const selectedProject = useMemo(
    () => projects.data?.find((project) => project.id === projectId),
    [projectId, projects.data],
  );
  const canManage =
    selectedProject?.role === "owner" || selectedProject?.role === "admin";
  const members = useProjectMembers(projectId || undefined);
  const invitations = useProjectInvitations(projectId || undefined, canManage);
  const updateMember = useUpdateProjectMember();
  const removeMember = useRemoveProjectMember();
  const revokeInvitation = useRevokeProjectInvitation();
  const inviteModal = useOverlayState();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (
      projects.data?.length &&
      !projects.data.some((project) => project.id === projectId)
    ) {
      setProjectId(projects.data[0].id);
    } else if (projects.data && !projects.data.length && projectId) {
      setProjectId("");
    }
  }, [projectId, projects.data]);

  async function changeRole(userId: string, role: AssignableRole) {
    setError(null);
    try {
      await updateMember.mutateAsync({ projectId, userId, role });
    } catch (cause) {
      setError(apiMessage(cause, "The member role could not be updated."));
    }
  }

  async function remove(userId: string, name: string) {
    if (!confirm(`Remove ${name} from this project?`)) return;
    setError(null);
    try {
      await removeMember.mutateAsync({ projectId, userId });
    } catch (cause) {
      setError(apiMessage(cause, "The member could not be removed."));
    }
  }

  async function revoke(invitationId: string, email: string) {
    if (!confirm(`Revoke the invitation for ${email}?`)) return;
    setError(null);
    try {
      await revokeInvitation.mutateAsync({ projectId, invitationId });
    } catch (cause) {
      setError(apiMessage(cause, "The invitation could not be revoked."));
    }
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={UsersRound}
        title="Team access"
        description="Give teammates the minimum project access they need."
        action={
          canManage ? (
            <Button variant="primary" onPress={inviteModal.open}>
              <UserPlus className="h-4 w-4" /> Invite teammate
            </Button>
          ) : undefined
        }
      />

      {error && <Alert message={error} />}

      {projects.isPending ? (
        <LoadingBlock />
      ) : !projects.data?.length ? (
        <EmptyBlock
          icon={UsersRound}
          title="No projects available"
          description="Create a project before adding teammates."
        />
      ) : (
        <>
          <section className="flex flex-col gap-4 border-b border-default-200 pb-6 sm:flex-row sm:items-end sm:justify-between">
            <label className="flex max-w-md flex-1 flex-col gap-1.5 text-sm">
              <span className="font-medium text-default-700">Project</span>
              <NativeSelect
                value={projectId}
                onChange={(event) => {
                  setError(null);
                  setProjectId(event.target.value);
                }}
              >
                {projects.data.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </NativeSelect>
            </label>
            {selectedProject && (
              <div className="flex items-center gap-3 text-sm">
                <RoleBadge role={selectedProject.role} />
                <span className="max-w-sm text-default-500">
                  {ROLE_COPY[selectedProject.role]}
                </span>
              </div>
            )}
          </section>

          <section aria-labelledby="members-title">
            <div className="mb-4 flex items-end justify-between gap-4">
              <div>
                <h2 id="members-title" className="text-lg font-semibold">
                  Members
                </h2>
                <p className="mt-1 text-sm text-default-500">
                  Access is scoped to {selectedProject?.name ?? "this project"}.
                </p>
              </div>
              <span className="text-sm tabular-nums text-default-500">
                {members.data?.length ?? 0} total
              </span>
            </div>

            {members.isPending ? (
              <LoadingBlock />
            ) : members.isError ? (
              <Alert message="Project members could not be loaded." />
            ) : (
              <div className="divide-y divide-default-100 overflow-hidden rounded-2xl border border-default-200 bg-background">
                {members.data?.map((member) => (
                  <div
                    key={member.user_id}
                    className="grid gap-4 px-4 py-4 transition-colors hover:bg-default-50 sm:grid-cols-[minmax(0,1fr)_auto_auto] sm:items-center"
                  >
                    <div className="flex min-w-0 items-center gap-3">
                      <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-default-100 text-sm font-semibold text-default-600">
                        {initials(member.name)}
                      </span>
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">
                          {member.name}
                        </p>
                        <p className="truncate text-xs text-default-500">
                          {member.email} · Joined{" "}
                          {new Date(member.joined_at).toLocaleDateString()}
                        </p>
                      </div>
                    </div>

                    {canManage && member.role !== "owner" ? (
                      <NativeSelect
                        aria-label={`Role for ${member.name}`}
                        value={member.role}
                        disabled={updateMember.isPending}
                        onChange={(event) =>
                          changeRole(
                            member.user_id,
                            event.target.value as AssignableRole,
                          )
                        }
                      >
                        <option value="admin">Admin</option>
                        <option value="editor">Editor</option>
                        <option value="viewer">Viewer</option>
                      </NativeSelect>
                    ) : (
                      <RoleBadge role={member.role} />
                    )}

                    {canManage && member.role !== "owner" ? (
                      <Button
                        size="sm"
                        variant="danger-soft"
                        aria-label={`Remove ${member.name}`}
                        onPress={() => remove(member.user_id, member.name)}
                        isPending={removeMember.isPending}
                      >
                        <Trash2 className="h-3.5 w-3.5" /> Remove
                      </Button>
                    ) : (
                      <span className="hidden w-24 sm:block" />
                    )}
                  </div>
                ))}
              </div>
            )}
          </section>

          {canManage && (
            <section aria-labelledby="invitations-title">
              <div className="mb-4">
                <h2 id="invitations-title" className="text-lg font-semibold">
                  Pending invitations
                </h2>
                <p className="mt-1 text-sm text-default-500">
                  Invite links expire after seven days and are shown only once.
                </p>
              </div>

              {invitations.isPending ? (
                <LoadingBlock />
              ) : invitations.isError ? (
                <Alert message="Pending invitations could not be loaded." />
              ) : !invitations.data?.length ? (
                <div className="rounded-2xl border border-dashed border-default-200 px-5 py-8 text-center text-sm text-default-500">
                  No pending invitations for this project.
                </div>
              ) : (
                <div className="divide-y divide-default-100 overflow-hidden rounded-2xl border border-default-200 bg-background">
                  {invitations.data.map((invitation) => (
                    <div
                      key={invitation.id}
                      className="flex flex-wrap items-center justify-between gap-4 px-4 py-4 transition-colors hover:bg-default-50"
                    >
                      <div className="flex min-w-0 items-center gap-3">
                        <Mail className="h-4 w-4 shrink-0 text-default-400" />
                        <div className="min-w-0">
                          <p className="truncate text-sm font-medium">
                            {invitation.email}
                          </p>
                          <p className="text-xs text-default-500">
                            Invited by {invitation.inviter_name} · Expires{" "}
                            {new Date(
                              invitation.expires_at,
                            ).toLocaleDateString()}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <RoleBadge role={invitation.role} />
                        <Button
                          size="sm"
                          variant="danger-soft"
                          onPress={() =>
                            revoke(invitation.id, invitation.email)
                          }
                          isPending={revokeInvitation.isPending}
                        >
                          Revoke
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>
          )}
        </>
      )}

      {projectId && (
        <InviteModal
          state={inviteModal}
          projectId={projectId}
          projectName={selectedProject?.name ?? "project"}
        />
      )}
    </div>
  );
}

function InviteModal({
  state,
  projectId,
  projectName,
}: {
  state: ReturnType<typeof useOverlayState>;
  projectId: string;
  projectName: string;
}) {
  const create = useCreateProjectInvitation();
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<AssignableRole>("viewer");
  const [created, setCreated] = useState<CreatedProjectInvitation | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!state.isOpen) {
      setEmail("");
      setRole("viewer");
      setCreated(null);
      setCopied(false);
      setError(null);
    }
  }, [state.isOpen]);

  function close() {
    state.close();
    setEmail("");
    setRole("viewer");
    setCreated(null);
    setCopied(false);
    setError(null);
  }

  async function submit() {
    setError(null);
    try {
      const invitation = await create.mutateAsync({ projectId, email, role });
      setCreated(invitation);
    } catch (cause) {
      setError(apiMessage(cause, "The invitation could not be created."));
    }
  }

  async function copyLink() {
    if (!created) return;
    try {
      await navigator.clipboard.writeText(created.accept_url);
      setCopied(true);
    } catch {
      setError("Copy failed. Select the link and copy it manually.");
    }
  }

  return (
    <FormModal
      state={state}
      title={created ? "Invitation ready" : "Invite teammate"}
      description={
        created
          ? "Copy this link now. FileBase stores only its hash."
          : `Grant access to ${projectName}.`
      }
    >
      {created ? (
        <div className="flex flex-col gap-4">
          <div className="rounded-2xl border border-success/30 bg-success/10 p-4">
            <div className="flex items-center gap-2 text-sm font-medium text-success">
              <Check className="h-4 w-4" /> Invitation created
            </div>
            <p className="mt-1 text-xs text-default-600">
              {created.email} will join as {created.role}.
            </p>
          </div>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium text-default-700">Invite link</span>
            <div className="flex gap-2">
              <input
                readOnly
                value={created.accept_url}
                className="min-w-0 flex-1 rounded-xl border border-default-200 bg-default-50 px-3 font-mono text-xs"
              />
              <Button variant="secondary" onPress={copyLink}>
                {copied ? (
                  <Check className="h-4 w-4" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
                {copied ? "Copied" : "Copy"}
              </Button>
            </div>
          </label>
          {error && <Alert message={error} />}
          <ModalActions>
            <Button variant="primary" onPress={close}>
              Done
            </Button>
          </ModalActions>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <TextField type="email" isRequired>
            <Label>Email</Label>
            <Input
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="teammate@company.com"
            />
          </TextField>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="font-medium text-default-700">Role</span>
            <NativeSelect
              value={role}
              onChange={(event) =>
                setRole(event.target.value as AssignableRole)
              }
            >
              <option value="viewer">Viewer</option>
              <option value="editor">Editor</option>
              <option value="admin">Admin</option>
            </NativeSelect>
            <span className="text-xs text-default-500">{ROLE_COPY[role]}</span>
          </label>
          {error && <Alert message={error} />}
          <ModalActions>
            <Button variant="tertiary" onPress={close}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onPress={submit}
              isPending={create.isPending}
              isDisabled={!email.trim()}
            >
              Send invitation
            </Button>
          </ModalActions>
        </div>
      )}
    </FormModal>
  );
}

function RoleBadge({ role }: { role: ProjectRole }) {
  return (
    <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full border border-default-200 bg-default-50 px-2.5 py-1 text-xs font-medium capitalize text-default-600">
      <ShieldCheck className="h-3.5 w-3.5 text-accent" />
      {role}
    </span>
  );
}

function initials(name: string) {
  return name
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toUpperCase();
}

function apiMessage(cause: unknown, fallback: string) {
  return cause instanceof ApiError ? cause.message : fallback;
}
