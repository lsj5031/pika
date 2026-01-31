import { Loader2 } from "lucide-react";
import { cn } from "../lib/utils";
import { Card } from "./ui/card";

interface ThinkingIndicatorProps {
  content: string;
  className?: string;
}

export function ThinkingIndicator({ content, className }: ThinkingIndicatorProps) {
  return (
    <div className={cn("flex w-full justify-start", className)}>
      <Card className="max-w-[80%] px-4 py-3 bg-thinking border-2 border-primary rotate-1 shadow-hard-sm">
        <div className="flex items-center gap-2">
          <Loader2 className="h-4 w-4 animate-spin text-primary" />
          <span className="text-xs font-bold text-thinking-foreground uppercase tracking-wide font-body">
            Thinking
          </span>
        </div>
        {content && (
          <p className="text-lg whitespace-pre-wrap break-words mt-2 text-thinking-foreground font-body leading-snug">
            {content}
          </p>
        )}
      </Card>
    </div>
  );
}
