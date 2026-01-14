import * as React from "react"

import { cn } from "../../lib/utils"

const Textarea = React.forwardRef<
  HTMLTextAreaElement,
  React.ComponentProps<"textarea">
>(({ className, ...props }, ref) => {
  return (
    <textarea
      className={cn(
        "flex min-h-[80px] w-full rounded-wobbly border-2 border-primary bg-background px-4 py-3 text-lg font-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:border-[#2d5da1] focus-visible:ring-2 focus-visible:ring-[#2d5da1]/20 disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
      ref={ref}
      {...props}
    />
  )
})
Textarea.displayName = "Textarea"

export { Textarea }
