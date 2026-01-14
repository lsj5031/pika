import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "../../lib/utils"

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-wobbly text-lg font-body font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-5 [&_svg]:shrink-0 active:scale-95 duration-100",
  {
    variants: {
      variant: {
        default:
          "bg-white text-primary border-[3px] border-primary shadow-hard hover:bg-accent hover:text-accent-foreground hover:shadow-hard-hover hover:-translate-y-0.5 hover:translate-x-0.5 active:shadow-none active:translate-x-1 active:translate-y-1 hover:rotate-1",
        destructive:
          "bg-white text-destructive border-[3px] border-destructive shadow-hard hover:bg-destructive hover:text-destructive-foreground hover:shadow-hard-hover active:shadow-none",
        outline:
          "border-[3px] border-primary bg-background hover:bg-muted hover:text-accent-foreground",
        secondary:
          "bg-muted text-secondary-foreground border-[3px] border-muted-foreground/20 hover:bg-secondary/80 shadow-sm",
        ghost: "hover:bg-accent hover:text-accent-foreground hover:shadow-hard-hover hover:border-[3px] border-transparent",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-12 px-6 py-2",
        sm: "h-10 rounded-wobbly px-4",
        lg: "h-14 rounded-wobbly px-10 text-xl",
        icon: "h-12 w-12",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
  VariantProps<typeof buttonVariants> {
  asChild?: boolean
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button"
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    )
  }
)
Button.displayName = "Button"

/* eslint-disable react-refresh/only-export-components */
export { Button, buttonVariants }
