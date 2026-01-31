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
            <div className="border-b md:border-b-0 md:border-r p-2 bg-diff-removed text-diff-removed-text">
              <div className="font-semibold mb-2 flex items-center gap-2">
                <span className="inline-block w-2 h-2 rounded-full bg-error"></span>
                Before
              </div>
              <pre className="whitespace-pre-wrap break-all opacity-80">
                {diff.oldContent || "// No previous content"}
              </pre>
            </div>
            {/* New content */}
            <div className="p-2 bg-diff-added text-diff-added-text">
              <div className="font-semibold mb-2 flex items-center gap-2">
                <span className="inline-block w-2 h-2 rounded-full bg-success"></span>
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
                  <div className="bg-error/20 px-1 py-0.5 rounded -mx-1 mb-2">
                    <span className="text-error line-through">
                      {diff.oldContent}
                    </span>
                  </div>
                  <div className="bg-success/20 px-1 py-0.5 rounded -mx-1">
                    <span className="text-success">
                      {diff.newContent}
                    </span>
                  </div>
                </>
              ) : (
                <div className={cn(
                  "p-2 rounded",
                  diff.newContent ? "bg-success/20" : "bg-error/20"
                )}>
                  <span className={diff.newContent ? "text-success-foreground" : "text-error-foreground"}>
                    {diff.newContent || diff.oldContent || "// No content"}
                  </span>
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
