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
      <Card className="max-w-[80%] px-4 py-2 bg-amber-50 dark:bg-amber-950/30 border-amber-200 dark:border-amber-800">
        <div className="flex items-center gap-2">
          <Loader2 className="h-4 w-4 animate-spin text-amber-600 dark:text-amber-500" />
          <span className="text-xs font-medium text-amber-700 dark:text-amber-400 uppercase tracking-wide">
            Thinking
          </span>
        </div>
        {content && (
          <p className="text-sm whitespace-pre-wrap break-words mt-2 text-amber-900 dark:text-amber-100 font-mono">
            {content}
          </p>
        )}
      </Card>
    </div>
  );
}
