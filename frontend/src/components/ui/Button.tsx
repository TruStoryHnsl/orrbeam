import type { ButtonHTMLAttributes } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "sunshine" | "moonlight" | "danger" | "ghost";
  size?: "sm" | "md";
}

const variants = {
  sunshine: "bg-sunshine/20 text-sunshine hover:bg-sunshine/30 border border-sunshine/30",
  moonlight: "bg-moonlight/20 text-moonlight hover:bg-moonlight/30 border border-moonlight/30",
  danger: "bg-red-500/20 text-red-400 hover:bg-red-500/30 border border-red-500/30",
  ghost: "text-neutral-400 hover:text-white hover:bg-surface-3",
};

const sizes = {
  sm: "px-2 py-1 text-xs",
  md: "px-3 py-1.5 text-sm",
};

export function Button({
  variant = "ghost",
  size = "md",
  className = "",
  disabled,
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      className={`rounded-md font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${variants[variant]} ${sizes[size]} ${className}`}
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
}
