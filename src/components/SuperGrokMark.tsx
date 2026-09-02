/** Zhimind logo used by the empty-session welcome surface. */

import { memo } from "react";

export type SuperGrokBrandKind = "supergrok" | "heavy";

export const SuperGrokMark = memo(function SuperGrokMark({
  kind,
  className = "",
  title,
}: {
  kind: SuperGrokBrandKind;
  className?: string;
  title?: string;
}) {
  const classes = [
    "supergrok-mark",
    kind === "heavy" ? "supergrok-mark--heavy" : "supergrok-mark--standard",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <span
      className={classes}
      role={title ? "img" : undefined}
      aria-hidden={title ? undefined : true}
      aria-label={title}
    >
      <span className="supergrok-mark__asset" aria-hidden />
    </span>
  );
});
