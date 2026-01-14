import { useState } from "react";
import { Card } from "./ui/card";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { ExternalLink, Code, FileDiff } from "lucide-react";
import { cn } from "../lib/utils";

interface DiffViewerProps {
  diff: {
    filePath?: string;
    oldContent?: string;
    newContent?: string;
    language?: string;
    diffUrl?: string;
  };
  className?: string;
}

export function DiffViewer({ diff, className }: DiffViewerProps) {
  const [viewMode, setViewMode] = useState<"split" | "unified">("split");

  if (!diff.oldContent && !diff.newContent) {
    return null;
  }

  return (
    <Card className={cn("p-4 space-y-3", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FileDiff className="h-4 w-4" />
          <Badge variant="outline" className="text-xs">
            {diff.filePath || "Unknown file"}
          </Badge>
          {diff.language && (
            <Badge variant="secondary" className="text-xs">
              <Code className="h-3 w-3 mr-1" />
              {diff.language}
            </Badge>
          )}
        </div>

        {/* View mode toggle */}
        <div className="flex gap-1">
          <Button
            variant={viewMode === "split" ? "default" : "outline"}
            size="sm"
            onClick={() => setViewMode("split")}
            className="h-7 text-xs"
          >
            Split
          </Button>
          <Button
            variant={viewMode === "unified" ? "default" : "outline"}
            size="sm"
            onClick={() => setViewMode("unified")}
            className="h-7 text-xs"
          >
            Unified
          </Button>
        </div>
      </div>

      {/* Diff content - simple line-by-line comparison */}
      <div className={cn(
        "border rounded-lg overflow-hidden text-xs font-mono",
        viewMode === "split" ? "grid grid-cols-2" : ""
      )}>
        {viewMode === "split" ? (
          <>
            {/* Old content */}
            <div className="border-r bg-red-50 dark:bg-red-950/20 p-2">
              <div className="font-semibold mb-2 text-red-700 dark:text-red-400">
                Before
              </div>
              <pre className="whitespace-pre-wrap break-words">
                {diff.oldContent || "// No previous content"}
              </pre>
            </div>
            {/* New content */}
            <div className="bg-green-50 dark:bg-green-950/20 p-2">
              <div className="font-semibold mb-2 text-green-700 dark:text-green-400">
                After
              </div>
              <pre className="whitespace-pre-wrap break-words">
                {diff.newContent || "// No new content"}
              </pre>
            </div>
          </>
        ) : (
          /* Unified view */
          <div className="p-2 bg-muted/30">
            <div className="font-semibold mb-2">Changes</div>
            <pre className="whitespace-pre-wrap break-words">
              {diff.oldContent && diff.newContent ? (
                <>
                  <span className="line-through text-red-600 dark:text-red-400">
                    {diff.oldContent}
                  </span>
                  {"\n\n"}
                  <span className="text-green-600 dark:text-green-400">
                    {diff.newContent}
                  </span>
                </>
              ) : diff.newContent || diff.oldContent || "// No content"}
            </pre>
          </div>
        )}
      </div>

      {/* External diff link */}
      {diff.diffUrl && (
        <div className="flex justify-end">
          <Button
            variant="ghost"
            size="sm"
            asChild
            className="h-7 text-xs"
          >
            <a
              href={diff.diffUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1"
            >
              <ExternalLink className="h-3 w-3" />
              View on diffs.com
            </a>
          </Button>
        </div>
      )}
    </Card>
  );
}
