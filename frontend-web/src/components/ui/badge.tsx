import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "../../lib/utils"

const badgeVariants = cva(
  "inline-flex items-center rounded-wobblyMd border-2 border-primary px-3 py-1 text-sm font-body font-bold transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 hover:rotate-2",
  {
    variants: {
      variant: {
        default:
          "bg-primary text-primary-foreground hover:bg-primary/80",
        secondary:
          "bg-thinking text-thinking-foreground hover:bg-thinking/80",
        destructive:
          "bg-destructive text-destructive-foreground hover:bg-destructive/80",
        outline: "text-foreground border-dashed bg-transparent",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
)

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
  VariantProps<typeof badgeVariants> { }

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  )
}

/* eslint-disable react-refresh/only-export-components */
export { Badge, badgeVariants }
