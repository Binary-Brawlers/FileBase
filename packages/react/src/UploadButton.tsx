import {
  forwardRef,
  useImperativeHandle,
  useRef,
  type ChangeEvent,
  type CSSProperties,
  type ReactNode,
} from "react";
import { useUpload, type UseUploadOptions, type UseUploadReturn } from "./useUpload";

export type UploadButtonProps = UseUploadOptions & {
  accept?: string;
  multiple?: boolean;
  maxSize?: number;
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode | ((state: UseUploadReturn) => ReactNode);
};

export const UploadButton = forwardRef<HTMLInputElement, UploadButtonProps>(function UploadButton(
  props,
  ref
) {
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
  const inputRef = useRef<HTMLInputElement | null>(null);
  useImperativeHandle(ref, () => inputRef.current as HTMLInputElement);

  const handleChange = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files ?? []);
    event.target.value = "";
    for (const file of files) {
      if (maxSize && file.size > maxSize) {
        const error = new Error(`file exceeds maxSize of ${maxSize} bytes`);
        onUploadError?.(Object.assign(error, { code: "validation_error" }) as never);
        continue;
      }
      await upload.upload(file);
      if (!multiple) break;
    }
  };

  return (
    <label className={className} style={style}>
      <input
        ref={inputRef}
        type="file"
        accept={accept}
        multiple={multiple}
        disabled={disabled || upload.isUploading}
        onChange={handleChange}
        style={{ display: "none" }}
      />
      {typeof children === "function" ? children(upload) : children ?? defaultLabel(upload)}
    </label>
  );
});

function defaultLabel(upload: UseUploadReturn): ReactNode {
  if (upload.isUploading) {
    const pct = upload.progress?.fraction != null ? Math.round(upload.progress.fraction * 100) : null;
    return pct != null ? `Uploading… ${pct}%` : "Uploading…";
  }
  if (upload.error) return `Upload failed — retry`;
  return "Upload";
}
