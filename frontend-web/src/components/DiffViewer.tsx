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
        viewMode === "split" ? "grid grid-cols-1 md:grid-cols-2" : ""
      )}>
        {viewMode === "split" ? (
          <>
            {/* Old content */}
            <div className="border-b md:border-b-0 md:border-r bg-red-50 dark:bg-red-950/20 p-2">
              <div className="font-semibold mb-2 text-red-700 dark:text-red-400 flex items-center gap-2">
                <span className="inline-block w-2 h-2 rounded-full bg-red-500"></span>
                Before
              </div>
              <pre className="whitespace-pre-wrap break-all opacity-80">
                {diff.oldContent || "// No previous content"}
              </pre>
            </div>
            {/* New content */}
            <div className="bg-green-50 dark:bg-green-950/20 p-2">
              <div className="font-semibold mb-2 text-green-700 dark:text-green-400 flex items-center gap-2">
                <span className="inline-block w-2 h-2 rounded-full bg-green-500"></span>
                After
              </div>
              <pre className="whitespace-pre-wrap break-all">
                {diff.newContent || "// No new content"}
              </pre>
            </div>
          </>
        ) : (
          /* Unified view */
          <div className="p-3 bg-muted/10">
            <div className="font-semibold mb-3 flex items-center gap-2">
              <FileDiff className="h-4 w-4" />
              Changes
            </div>
            <pre className="whitespace-pre-wrap break-all leading-relaxed">
              {diff.oldContent && diff.newContent ? (
                <>
                  <div className="bg-red-100/50 dark:bg-red-900/20 px-1 py-0.5 rounded -mx-1 mb-2">
                    <span className="text-red-600 dark:text-red-400 line-through">
                      {diff.oldContent}
                    </span>
                  </div>
                  <div className="bg-green-100/50 dark:bg-green-900/20 px-1 py-0.5 rounded -mx-1">
                    <span className="text-green-600 dark:text-green-400">
                      {diff.newContent}
                    </span>
                  </div>
                </>
              ) : (
                <div className={cn(
                  "p-2 rounded",
                  diff.newContent ? "bg-green-50 dark:bg-green-900/20 text-green-700" : "bg-red-50 dark:bg-red-900/20 text-red-700"
                )}>
                  {diff.newContent || diff.oldContent || "// No content"}
                </div>
              )}
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
