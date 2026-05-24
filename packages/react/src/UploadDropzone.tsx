import {
  useCallback,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent,
  type ReactNode,
} from "react";
import { useUpload, type UseUploadOptions, type UseUploadReturn } from "./useUpload";

export type UploadDropzoneProps = UseUploadOptions & {
  accept?: string;
  multiple?: boolean;
  maxSize?: number;
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode | ((state: UseUploadReturn & { isDragging: boolean }) => ReactNode);
};

export function UploadDropzone(props: UploadDropzoneProps) {
  const {
    accept,
    multiple,
    maxSize,
    disabled,
    className,
    style,
    children,
    onUploadError,
    ...uploadOptions
  } = props;
  const upload = useUpload({ ...uploadOptions, onUploadError });
  const [isDragging, setIsDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement | null>(null);

  const acceptList = accept
    ? accept.split(",").map((s) => s.trim().toLowerCase()).filter(Boolean)
    : null;

  const matchesAccept = useCallback(
    (file: File) => {
      if (!acceptList || acceptList.length === 0) return true;
      const name = file.name.toLowerCase();
      const type = file.type.toLowerCase();
      return acceptList.some((rule) => {
        if (rule.startsWith(".")) return name.endsWith(rule);
        if (rule.endsWith("/*")) return type.startsWith(rule.slice(0, -1));
        return type === rule;
      });
    },
    [acceptList]
  );

  const processFiles = useCallback(
    async (files: File[]) => {
      for (const file of files) {
        if (!matchesAccept(file)) continue;
        if (maxSize && file.size > maxSize) continue;
        await upload.upload(file);
        if (!multiple) break;
      }
    },
    [matchesAccept, maxSize, multiple, upload]
  );

  const handleDrop = useCallback(
    async (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      setIsDragging(false);
      if (disabled || upload.isUploading) return;
      const files = Array.from(event.dataTransfer.files ?? []);
      await processFiles(files);
    },
    [disabled, upload.isUploading, processFiles]
  );

  return (
    <div
      className={className}
      style={style}
      onDragOver={(event) => {
        event.preventDefault();
        if (!disabled) setIsDragging(true);
      }}
      onDragLeave={(event) => {
        event.preventDefault();
        setIsDragging(false);
      }}
      onDrop={handleDrop}
      onClick={() => !disabled && !upload.isUploading && inputRef.current?.click()}
      role="button"
      tabIndex={disabled ? -1 : 0}
      aria-disabled={disabled || upload.isUploading}
    >
      <input
        ref={inputRef}
        type="file"
        accept={accept}
        multiple={multiple}
        disabled={disabled || upload.isUploading}
        style={{ display: "none" }}
        onChange={async (event) => {
          const files = Array.from(event.target.files ?? []);
          event.target.value = "";
          await processFiles(files);
        }}
      />
      {typeof children === "function"
        ? children({ ...upload, isDragging })
        : children ?? defaultContent({ ...upload, isDragging })}
    </div>
  );
}

function defaultContent(state: UseUploadReturn & { isDragging: boolean }): ReactNode {
  if (state.isUploading) {
    const pct = state.progress?.fraction != null ? Math.round(state.progress.fraction * 100) : null;
    return pct != null ? `Uploading… ${pct}%` : "Uploading…";
  }
  if (state.error) return "Upload failed — drop or click to retry";
  if (state.isDragging) return "Drop the file to upload";
  return "Drag a file here or click to upload";
}
