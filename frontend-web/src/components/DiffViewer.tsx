import { useState } from "react";
import { MultiFileDiff } from "@pierre/diffs/react";
import type { FileContents } from "@pierre/diffs/react";
import { Card } from "./ui/card";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { Code, FileDiff } from "lucide-react";
import { cn } from "../lib/utils";
import { useTheme } from "next-themes";
import type { BundledLanguage, BundledTheme } from "shiki";

interface DiffViewerProps {
  diff: {
    filePath?: string;
    oldContent?: string;
    newContent?: string;
    language?: string;
  };
  className?: string;
}

const LANGUAGE_MAP: Record<string, BundledLanguage> = {
  typescript: "typescript",
  javascript: "javascript",
  jsx: "jsx",
  tsx: "tsx",
  rust: "rust",
  python: "python",
  go: "go",
  json: "json",
  html: "html",
  css: "css",
  markdown: "markdown",
  yaml: "yaml",
  bash: "bash",
  shell: "bash",
  sh: "bash",
  dockerfile: "dockerfile",
  sql: "sql",
  toml: "toml",
};

export function DiffViewer({ diff, className }: DiffViewerProps) {
  const [viewMode, setViewMode] = useState<"split" | "unified">("split");
  const { theme } = useTheme();
  
  const isDark = theme === "dark" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  const shikiTheme: BundledTheme = isDark ? "github-dark" : "github-light";

  if (!diff.oldContent && !diff.newContent) {
    return null;
  }

  const lang = diff.language?.toLowerCase() || "typescript";
  const mappedLang = LANGUAGE_MAP[lang] || "typescript";

  const oldFile: FileContents = {
    name: diff.filePath || "old",
    contents: diff.oldContent || "",
    lang: mappedLang,
  };

  const newFile: FileContents = {
    name: diff.filePath || "new",
    contents: diff.newContent || "",
    lang: mappedLang,
  };

  return (
    <Card className={cn("flex flex-col overflow-hidden", className)}>
      <div className="flex items-center justify-between p-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <FileDiff className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium truncate max-w-[200px] md:max-w-[400px]">
            {diff.filePath || "Unknown file"}
          </span>
          {diff.language && (
            <Badge variant="secondary" className="text-xs h-5 px-1.5 gap-1 font-normal">
              <Code className="h-3 w-3" />
              {diff.language}
            </Badge>
          )}
        </div>

        <div className="flex bg-muted/50 p-0.5 rounded-lg border">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setViewMode("split")}
            className={cn(
              "h-6 px-2 text-xs rounded-md hover:bg-background hover:text-foreground",
              viewMode === "split" && "bg-background text-foreground shadow-sm"
            )}
          >
            Split
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setViewMode("unified")}
            className={cn(
              "h-6 px-2 text-xs rounded-md hover:bg-background hover:text-foreground",
              viewMode === "unified" && "bg-background text-foreground shadow-sm"
            )}
          >
            Unified
          </Button>
        </div>
      </div>

      <div className="relative text-xs [&_.d-diff-table]:font-mono overflow-hidden">
        <MultiFileDiff
          oldFile={oldFile}
          newFile={newFile}
          options={{
            diffStyle: viewMode,
            theme: shikiTheme,
            diffIndicators: "classic",
          }}
          style={{
            "--d-border": "var(--border)",
            "--d-background": "var(--card)",
            "--d-text-primary": "var(--foreground)",
            "--d-text-secondary": "var(--muted-foreground)",
            "--d-line-number-color": "var(--muted-foreground)",
            "--d-line-added-bg": "var(--diff-added-bg)",
            "--d-line-removed-bg": "var(--diff-removed-bg)",
            "--d-line-added-text": "var(--diff-added-text)",
            "--d-line-removed-text": "var(--diff-removed-text)",
          } as React.CSSProperties}
        />
      </div>
    </Card>
  );
}
