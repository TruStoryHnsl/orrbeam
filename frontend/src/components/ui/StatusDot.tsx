const colors = {
  online: "bg-green-500",
  running: "bg-green-500",
  installed: "bg-yellow-500",
  offline: "bg-neutral-500",
  not_installed: "bg-red-500",
  unknown: "bg-neutral-500",
  hosting: "bg-sunshine",
  connected: "bg-moonlight",
} as const;

type StatusType = keyof typeof colors;

export function StatusDot({ status }: { status: StatusType }) {
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${colors[status] ?? "bg-neutral-500"}`}
    />
  );
}
