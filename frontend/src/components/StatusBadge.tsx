import type { Status } from "../format";
import { AlertIcon, CheckIcon, ClockIcon, PauseIcon } from "./Icons";

const LABELS: Record<Status, string> = {
  reached: "Target reached",
  tracking: "Tracking",
  error: "Check failed",
  paused: "Paused",
  pending: "First check…",
};

/** Status is always icon + text, never color alone. */
export function StatusBadge({ status }: { status: Status }) {
  const icon =
    status === "reached" ? <CheckIcon /> :
    status === "error" ? <AlertIcon /> :
    status === "paused" ? <PauseIcon /> :
    status === "pending" ? <ClockIcon /> :
    <span className="pulse" />;
  return (
    <span className={`badge badge-${status}`}>
      {icon}
      {LABELS[status]}
    </span>
  );
}
