type Props = {
  size?: "md" | "lg";
  showWordmark?: boolean;
  tagline?: string;
};

export function Brand({ size = "md", showWordmark = true, tagline }: Props) {
  return (
    <div className="flex items-center gap-3">
      <span
        className={
          "brand-mark" + (size === "lg" ? " brand-mark--lg" : "")
        }
        aria-hidden
      >
        FB
      </span>
      {showWordmark && (
        <div className="flex flex-col leading-tight">
          <span
            className={
              "font-semibold tracking-tight " +
              (size === "lg" ? "text-xl" : "text-base")
            }
          >
            FileBase
          </span>
          {tagline && (
            <span className="text-xs text-default-500">{tagline}</span>
          )}
        </div>
      )}
    </div>
  );
}
