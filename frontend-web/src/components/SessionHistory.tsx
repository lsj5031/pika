import { useEffect, useRef, useState } from "react";
import { useSessionHistory } from "../hooks/useSessionHistory";
import { useThinkingStore } from "../store/thinkingStore";
import { Card } from "./ui/card";
import { ScrollArea } from "./ui/scroll-area";
import { ThinkingIndicator } from "./ThinkingIndicator";
import { DiffViewer } from "./DiffViewer";
import { cn } from "../lib/utils";
import type { Message } from "../types";
import { Bot, User, Wrench, ChevronDown, ChevronUp } from "lucide-react";

interface SessionHistoryProps {
  sessionId: string | null;
  className?: string;
}

function formatTimestamp(timestamp: string | null): string {
  if (!timestamp) return "";
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function parseDiffFromMessage(content: string) {
  // Strip "Tool Result:" or "Tool Result (Error):" prefix
  let cleanContent = content.replace(/^Tool Result(\s+\(Error\))?:\s*/, "");

  // Strip "Tool Call: name(...)" wrapper to get inner JSON
  const toolCallMatch = cleanContent.match(/^Tool Call:\s*\w+\(([\s\S]*)\)\s*$/);
  if (toolCallMatch) {
    cleanContent = toolCallMatch[1];
  } else {
    // Fallback for lowercase "Tool call:" prefix
    cleanContent = cleanContent.replace(/^Tool call:\s*/, "");
  }

  cleanContent = cleanContent.trim();

  // Pattern 1: Markdown code blocks
  const fileBlockRegex = /```(\w+)?\n(?:\/\/ (.+?)\n)?([\s\S]*?)```/g;
  const matches = [...cleanContent.matchAll(fileBlockRegex)];

  if (matches.length >= 2) {
    return {
      filePath: matches[0][2] || undefined,
      language: matches[0][1] || "text",
      oldContent: matches[0][3]?.trim(),
      newContent: matches[1][3]?.trim(),
    };
  }

  // Pattern 2: Tool call JSON (e.g. from replace_file_content)
  try {
    if (cleanContent.startsWith("{") || cleanContent.startsWith("[")) {
      const data = JSON.parse(cleanContent);
      // If it's an array, take the first item if it looks like an object
      const root = Array.isArray(data) ? data[0] : data;

      // Check for replacement_content or code_content patterns
      const args = root.function?.arguments ? JSON.parse(root.function.arguments) : root.arguments || root;

      if (args.ReplacementContent && args.TargetContent) {
        return {
          filePath: args.TargetFile || undefined,
          language: args.TargetFile?.split('.').pop() || "text",
          oldContent: args.TargetContent,
          newContent: args.ReplacementContent,
        };
      }
      if (args.CodeContent && args.TargetFile) {
        return {
          filePath: args.TargetFile,
          language: args.TargetFile.split('.').pop() || "text",
          oldContent: "", // New file creation
          newContent: args.CodeContent,
        };
      }
    }
  } catch {
    // Ignore parse errors
  }

  return null;
}

const COLLAPSE_THRESHOLD = 12; // Lines before we collapse
const PREVIEW_LINES = 3; // Lines to show at start and end when collapsed

function truncateContent(content: string): { truncated: string; isTruncated: boolean } {
  const lines = content.split("\n");
  if (lines.length <= COLLAPSE_THRESHOLD) {
    return { truncated: content, isTruncated: false };
  }
  const firstLines = lines.slice(0, PREVIEW_LINES).join("\n");
  const lastLines = lines.slice(-PREVIEW_LINES).join("\n");
  const hiddenCount = lines.length - PREVIEW_LINES * 2;
  return {
    truncated: `${firstLines}\n\n... ${hiddenCount} more lines ...\n\n${lastLines}`,
    isTruncated: true,
  };
}

function parseThinkingBlocks(content: string): { thinking: string; response: string } {
  // Parse thinking blocks in format: <thinking>content</thinking>
  const thinkingRegex = /<thinking>([\s\S]*?)<\/thinking>/g;
  const matches = [...content.matchAll(thinkingRegex)];

  if (matches.length > 0) {
    const thinking = matches.map(m => m[1].trim()).join("\n\n");
    const response = content.replace(thinkingRegex, "").trim();
    return { thinking, response };
  }

  return { thinking: "", response: content };
}

function getMessageColors(role: string, hasToolUse: boolean) {
  if (role === "user") {
    return {
      bg: "bg-gradient-to-br from-blue-500 to-blue-600",
      text: "text-white",
      icon: "text-blue-100",
      border: "border-blue-400"
    };
  }

  if (hasToolUse) {
    return {
      bg: "bg-gradient-to-br from-amber-50 to-orange-50 dark:from-amber-950/30 dark:to-orange-950/30",
      text: "text-amber-900 dark:text-amber-100",
      icon: "text-amber-600 dark:text-amber-400",
      border: "border-amber-300 dark:border-amber-700"
    };
  }

  return {
    bg: "bg-gradient-to-br from-emerald-50 to-teal-50 dark:from-emerald-950/30 dark:to-teal-950/30",
    text: "text-emerald-900 dark:text-emerald-100",
    icon: "text-emerald-600 dark:text-emerald-400",
    border: "border-emerald-300 dark:border-emerald-700"
  };
}

function MessageBubble({ message }: { message: Message }) {
  const [isExpanded, setIsExpanded] = useState(false);
  const isUser = message.role === "user";
  const diff = !isUser ? parseDiffFromMessage(message.content) : null;
  const { thinking, response } = !isUser ? parseThinkingBlocks(message.content) : { thinking: "", response: message.content };
  const hasToolUse = message.content.includes("tool_use") || /\bTool (Call|Result|call)\b/.test(message.content);
  const colors = getMessageColors(message.role, hasToolUse);
  const showThinking = thinking && thinking.length > 0;
  const { truncated: truncatedResponse, isTruncated } = truncateContent(response);
  const displayResponse = isExpanded ? response : truncatedResponse;

  return (
    <div
      className={cn(
        "flex w-full flex-col gap-2 animate-in fade-in slide-in-from-bottom-2 duration-300",
        isUser ? "items-end" : "items-start"
      )}
    >
      {/* Role indicator */}
      <div className={cn("flex items-center gap-1.5 px-2 text-xs font-semibold", isUser ? "flex-row-reverse" : "flex-row")}>
        {isUser ? (
          <>
            <span className={cn(colors.icon)}>You</span>
            <User className="h-3.5 w-3.5" />
          </>
        ) : hasToolUse ? (
          <>
            <Wrench className="h-3.5 w-3.5" />
            <span className={cn(colors.icon)}>Tool Use</span>
          </>
        ) : (
          <>
            <Bot className="h-3.5 w-3.5" />
            <span className={cn(colors.icon)}>Assistant</span>
          </>
        )}
      </div>

      {/* Main message card */}
      <Card
        className={cn(
          "max-w-[92%] md:max-w-[80%] px-4 py-3 border-2 shadow-sm transition-all hover:shadow-md",
          colors.bg,
          colors.border,
          "overflow-hidden min-w-0" // Ensure nothing leaks out
        )}
      >
        {/* Response content */}
        {response && (
          <div className={cn(
            "text-sm whitespace-pre-wrap break-all md:break-words leading-relaxed",
            colors.text
          )}>
            {(/^Tool (Result|Call|call)/i.test(displayResponse)) && (displayResponse.includes("{") || displayResponse.includes("[")) ? (
              <div className="space-y-2">
                <span className="font-bold opacity-70 block mb-1">
                  {displayResponse.split(":")[0]}:
                </span>
                <pre className="bg-black/5 dark:bg-black/20 p-2 rounded border border-current/10 overflow-x-auto text-[10px] font-mono whitespace-pre">
                  {(() => {
                    // Find first JSON delimiter (either { or [)
                    const firstBrace = displayResponse.indexOf("{");
                    const firstBracket = displayResponse.indexOf("[");
                    const indices = [firstBrace, firstBracket].filter(i => i !== -1);
                    const firstJsonIdx = indices.length > 0 ? Math.min(...indices) : -1;

                    if (firstJsonIdx === -1) return displayResponse;

                    let jsonPart = displayResponse.slice(firstJsonIdx).trim();
                    // Strip trailing ) from "Tool Call: name({...})" format
                    jsonPart = jsonPart.replace(/\)\s*$/, "");

                    try {
                      const parsed = JSON.parse(jsonPart);
                      const formatted = JSON.stringify(parsed, null, 2);
                      // Apply truncation to formatted JSON if collapsed
                      if (!isExpanded) {
                        const { truncated, isTruncated: jsonTruncated } = truncateContent(formatted);
                        return jsonTruncated ? truncated : formatted;
                      }
                      return formatted;
                    } catch {
                      return jsonPart;
                    }
                  })()}
                </pre>
              </div>
            ) : displayResponse.trim().startsWith("{") || displayResponse.trim().startsWith("[") ? (
              <pre className="bg-black/5 dark:bg-black/20 p-2 rounded border border-current/10 overflow-x-auto text-[10px] font-mono whitespace-pre">
                {(() => {
                  try {
                    const parsed = JSON.parse(displayResponse);
                    const formatted = JSON.stringify(parsed, null, 2);
                    if (!isExpanded) {
                      const { truncated, isTruncated: jsonTruncated } = truncateContent(formatted);
                      return jsonTruncated ? truncated : formatted;
                    }
                    return formatted;
                  } catch {
                    return displayResponse;
                  }
                })()}
              </pre>
            ) : (
              displayResponse
            )}

            {/* Expand/Collapse button */}
            {isTruncated && (
              <button
                onClick={() => setIsExpanded(!isExpanded)}
                className={cn(
                  "mt-2 flex items-center gap-1 text-xs font-medium opacity-70 hover:opacity-100 transition-opacity",
                  colors.text
                )}
              >
                {isExpanded ? (
                  <>
                    <ChevronUp className="h-3 w-3" />
                    Show less
                  </>
                ) : (
                  <>
                    <ChevronDown className="h-3 w-3" />
                    Show more
                  </>
                )}
              </button>
            )}
          </div>
        )}

        {/* Thinking block - styled distinctly */}
        {showThinking && (
          <div className="mt-3 pt-3 border-t-2 border-dashed border-current opacity-80">
            <details className="group">
              <summary className="cursor-pointer text-xs font-bold uppercase tracking-wider mb-2 flex items-center gap-2 hover:opacity-80 transition-opacity">
                <span className="inline-block w-2 h-2 rounded-full bg-current animate-pulse"></span>
                Thinking Process
              </summary>
              <div className={cn("text-xs whitespace-pre-wrap break-words leading-relaxed pl-3 border-l-2 border-current opacity-90", colors.text)}>
                {thinking}
              </div>
            </details>
          </div>
        )}

        {/* Timestamp */}
        {message.timestamp && (
          <p
            className={cn(
              "text-xs mt-2 pt-2 border-t border-current/20",
              colors.text,
              "opacity-60"
            )}
          >
            {formatTimestamp(message.timestamp)}
          </p>
        )}
      </Card>

      {/* Show diff viewer if code changes detected */}
      {diff && (
        <DiffViewer
          diff={diff}
          className="max-w-[92%] md:max-w-[85%]"
        />
      )}
    </div>
  );
}

// Empty thinking state as a constant to avoid infinite loops
const EMPTY_THINKING_STATE = { content: "", isThinking: false } as const;

export function SessionHistory({ sessionId, className }: SessionHistoryProps) {
  const { data: messages, isLoading } = useSessionHistory({ sessionId });
  const scrollRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const thinkingState = useThinkingStore((state) =>
    sessionId ? state.thinkingBySession[sessionId] ?? EMPTY_THINKING_STATE : EMPTY_THINKING_STATE
  );
  const prevSessionIdRef = useRef<string | null>(null);
  const initialScrollDoneRef = useRef<boolean>(false);

  // Auto-scroll to bottom when new messages arrive or thinking updates
  useEffect(() => {
    // If session ID changed, reset scroll flag
    if (sessionId !== prevSessionIdRef.current) {
      prevSessionIdRef.current = sessionId;
      initialScrollDoneRef.current = false;
    }

    if (!messages) return;

    const shouldScroll = !initialScrollDoneRef.current || thinkingState.isThinking;

    if (shouldScroll) {
      // Use requestAnimationFrame to ensure DOM is updated
      const scroll = () => {
        messagesEndRef.current?.scrollIntoView({
          behavior: initialScrollDoneRef.current ? "smooth" : "auto"
        });
        if (!initialScrollDoneRef.current && messages.length > 0) {
          initialScrollDoneRef.current = true;
        }
      };

      requestAnimationFrame(scroll);
    }
  }, [messages, thinkingState, sessionId]);

  if (!sessionId) {
    return (
      <div
        className={cn(
          "flex items-center justify-center h-full text-muted-foreground",
          className
        )}
      >
        <p>Select a session to view history</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div
        className={cn(
          "flex items-center justify-center h-full text-muted-foreground",
          className
        )}
      >
        <p>Loading messages...</p>
      </div>
    );
  }

  if (!messages || messages.length === 0) {
    return (
      <div
        className={cn(
          "flex items-center justify-center h-full text-muted-foreground",
          className
        )}
      >
        <p>No messages yet</p>
      </div>
    );
  }

  return (
    <div className={cn("flex flex-col h-full", className)}>
      <ScrollArea className="flex-1">
        <div ref={scrollRef} className="p-4 space-y-6">
          {messages.map((message, index) => (
            <MessageBubble key={index} message={message} />
          ))}
          {thinkingState.isThinking && (
            <ThinkingIndicator content={thinkingState.content} />
          )}
          <div ref={messagesEndRef} />
        </div>
      </ScrollArea>
    </div>
  );
}
