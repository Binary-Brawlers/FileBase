"use client";

import {
  Button,
  FieldError,
  Input,
  Label,
  ListBox,
  Select,
  TextField,
  type Key,
} from "@heroui/react";

export type PresetData = {
  name: string;
  folder: string;
  max_file_size: number;
  duplicate_strategy: "return_existing" | "upload_new_copy" | "reject_duplicate";
};

type Props = {
  value: PresetData;
  onChange: (v: PresetData) => void;
  submitting: boolean;
  onBack: () => void;
  onSubmit: () => void;
};

const STRATEGIES: { key: PresetData["duplicate_strategy"]; label: string }[] = [
  { key: "return_existing", label: "Return existing file" },
  { key: "upload_new_copy", label: "Upload as new copy" },
  { key: "reject_duplicate", label: "Reject duplicate" },
];

export function PresetStep({
  value,
  onChange,
  submitting,
  onBack,
  onSubmit,
}: Props) {
  const set = (patch: Partial<PresetData>) => onChange({ ...value, ...patch });
  const valid =
    value.name.trim() && value.folder.trim() && value.max_file_size > 0;

  return (
    <div className="flex flex-col gap-5">
      <p className="text-sm text-default-500">
        Presets are reusable upload rules. You can edit or add more later.
      </p>

      <TextField isRequired>
        <Label>Name</Label>
        <Input
          value={value.name}
          onChange={(event) => set({ name: event.target.value })}
        />
        <FieldError />
      </TextField>
      <TextField isRequired>
        <Label>Folder</Label>
        <Input
          value={value.folder}
          onChange={(event) => set({ folder: event.target.value })}
        />
        <FieldError />
      </TextField>
      <TextField type="number">
        <Label>Max file size (bytes)</Label>
        <Input
          value={value.max_file_size.toString()}
          onChange={(event) =>
            set({ max_file_size: Number(event.target.value) || 0 })
          }
        />
        <FieldError />
      </TextField>
      <Select
        value={value.duplicate_strategy}
        onChange={(key: Key | null) => {
          if (typeof key === "string") {
            set({ duplicate_strategy: key as PresetData["duplicate_strategy"] });
          }
        }}
      >
        <Label>Duplicate strategy</Label>
        <Select.Trigger>
          <Select.Value />
          <Select.Indicator />
        </Select.Trigger>
        <Select.Popover>
          <ListBox>
            {STRATEGIES.map((s) => (
              <ListBox.Item key={s.key} id={s.key} textValue={s.label}>
                <Label>{s.label}</Label>
                <ListBox.ItemIndicator />
              </ListBox.Item>
            ))}
          </ListBox>
        </Select.Popover>
      </Select>

      <div className="flex justify-end gap-2">
        <Button variant="tertiary" onPress={onBack} isDisabled={submitting}>
          Back
        </Button>
        <Button
          variant="primary"
          onPress={onSubmit}
          isPending={submitting}
          isDisabled={!valid}
        >
          Finish setup
        </Button>
      </div>
    </div>
  );
}
