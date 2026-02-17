import { useEffect, useRef, useState, memo, useCallback, useMemo, lazy, Suspense } from "react";
import { useSessionHistory } from "../hooks/useSessionHistory";
import { useAppStore } from "../store/appStore";
import { useThinkingStore } from "../store/thinkingStore";
import { Card } from "./ui/card";
import { ScrollArea } from "./ui/scroll-area";
import { ThinkingIndicator } from "./ThinkingIndicator";
const LazyDiffViewer = lazy(() => import("./DiffViewer").then(module => ({ default: module.DiffViewer })));
const LazyReactMarkdown = lazy(() => import("react-markdown"));
import { Button } from "./ui/button";
import { cn } from "../lib/utils";
import { parseDiffFromMessage } from "../lib/diff-parser";
import type { Message } from "../types";
import { Bot, User, Wrench, ChevronDown, ChevronUp, Download, Loader2, MessageSquare } from "lucide-react";

interface SessionHistoryProps {
  sessionId: string | null;
  className?: string;
}

function formatTimestamp(timestamp: string | null): string {
  if (!timestamp) return "";
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
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
      bg: "bg-info",
      text: "text-info-foreground",
      icon: "text-info-foreground",
      border: "border-info"
    };
  }

  if (hasToolUse) {
    return {
      bg: "bg-warning/10 dark:bg-warning/20",
      text: "text-warning-foreground",
      icon: "text-warning",
      border: "border-warning/50"
    };
  }

  return {
    bg: "bg-success/10 dark:bg-success/20",
    text: "text-success-foreground",
    icon: "text-success",
    border: "border-success/50"
  };
}

// Memoized message bubble to prevent re-renders when other messages update
const MessageBubble = memo(function MessageBubble({ message }: { message: Message }) {
  const [isExpanded, setIsExpanded] = useState(false);
  const isUser = message.role === "user";

  // Memoize expensive parsing operations
  const { diff, thinking, response, hasToolUse, colors, isTruncated, displayResponse } = useMemo(() => {
    const diffResult = !isUser ? parseDiffFromMessage(message.content) : null;
    const { thinking: thinkingContent, response: responseContent } = !isUser
      ? parseThinkingBlocks(message.content)
      : { thinking: "", response: message.content };
    const hasToolUseContent = message.content.includes("tool_use") || /\bTool (Call|Result|call)\b/i.test(message.content);
    const colorSet = getMessageColors(message.role, hasToolUseContent);

    const { truncated: truncatedResponse, isTruncated: textIsTruncated } = truncateContent(responseContent);
    const displayResp = isExpanded ? responseContent : truncatedResponse;

    // Check if JSON content would be truncated
    let jsonIsTruncated = false;
    if (hasToolUseContent) {
      try {
        const firstBrace = responseContent.indexOf("{");
        const firstBracket = responseContent.indexOf("[");
        const indices = [firstBrace, firstBracket].filter(i => i !== -1);
        const firstJsonIdx = indices.length > 0 ? Math.min(...indices) : -1;
        if (firstJsonIdx !== -1) {
          const jsonPart = responseContent.slice(firstJsonIdx).trim().replace(/\)\s*$/, "");
          const parsed = JSON.parse(jsonPart);
          const formatted = JSON.stringify(parsed, null, 2);
          jsonIsTruncated = formatted.split("\n").length > COLLAPSE_THRESHOLD;
        }
      } catch {
        // Ignore parse errors
      }
    }

    return {
      diff: diffResult,
      thinking: thinkingContent,
      response: responseContent,
      hasToolUse: hasToolUseContent,
      colors: colorSet,
      isTruncated: textIsTruncated || jsonIsTruncated,
      displayResponse: displayResp
    };
  }, [message.content, message.role, isExpanded, isUser]);

  const showThinking = thinking && thinking.length > 0;

  // Memoize JSON formatting to prevent re-parsing on re-renders
  const formattedJson = useMemo(() => {
    if (!(/^Tool (Result|Call|Tool call)/i.test(displayResponse)) &&
        !(displayResponse.trim().startsWith("{") || displayResponse.trim().startsWith("["))) {
      return null;
    }

    const firstBrace = displayResponse.indexOf("{");
    const firstBracket = displayResponse.indexOf("[");
    const indices = [firstBrace, firstBracket].filter(i => i !== -1);
    const firstJsonIdx = indices.length > 0 ? Math.min(...indices) : -1;

    if (firstJsonIdx === -1) return null;

    let jsonPart = displayResponse.slice(firstJsonIdx).trim();
    jsonPart = jsonPart.replace(/\)\s*$/, "");

    try {
      const parsed = JSON.parse(jsonPart);
      const formatted = JSON.stringify(parsed, null, 2);
      if (!isExpanded) {
        const { truncated, isTruncated: jsonTruncated } = truncateContent(formatted);
        return jsonTruncated ? truncated : formatted;
      }
      return formatted;
    } catch {
      return jsonPart;
    }
  }, [displayResponse, isExpanded]);

  return (
    <div
      className={cn(
        "flex w-full max-w-full flex-col gap-2 animate-in fade-in slide-in-from-bottom-2 duration-300 min-w-0",
        isUser ? "items-end px-4" : "items-start px-4"
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

      {/* Image attachments - shown before text for user messages */}
      {message.images && message.images.length > 0 && (
        <div
          className={cn(
            "grid gap-2 mb-3 max-w-[85%] md:max-w-[80%]",
            message.images.length > 1 ? "grid-cols-2" : "grid-cols-1"
          )}
        >
          {message.images.map((img) => (
            <a
              key={img.id}
              href={img.url}
              target="_blank"
              rel="noopener noreferrer"
              className="block group"
            >
              <img
                src={img.url}
                alt={img.filename}
                className="rounded-lg border border-border w-full object-cover 
                         max-h-64 hover:opacity-90 transition-opacity"
                loading="lazy"
              />
              <span className="text-xs text-muted-foreground opacity-0 group-hover:opacity-70 
                            transition-opacity mt-1 block">
                {img.filename} ({(img.size / 1024).toFixed(1)} KB)
              </span>
            </a>
          ))}
        </div>
      )}

      {/* Main message card */}
      <Card
        className={cn(
          "max-w-full sm:max-w-[85%] px-4 py-3 border-2 shadow-sm transition-all hover:shadow-md overflow-visible !overflow-visible",
          colors.bg,
          colors.border,
          "min-w-0"
        )}
      >
        {/* Response content */}
        {response && (
          <div className={cn(
            "text-sm whitespace-pre-wrap break-words leading-relaxed",
            colors.text
          )}>
            {formattedJson ? (
              <div className="space-y-2">
                <span className="font-bold opacity-70 block mb-1">
                  {displayResponse.split(":")[0]}:
                </span>
                <pre className="bg-black/5 dark:bg-black/20 p-2 rounded border border-current/10 overflow-x-auto text-[10px] font-mono whitespace-pre-wrap break-all max-w-full">
                  {formattedJson}
                </pre>
              </div>
            ) : displayResponse.trim().startsWith("{") || displayResponse.trim().startsWith("[") ? (
              <pre className="bg-black/5 dark:bg-black/20 p-2 rounded border border-current/10 overflow-x-auto text-[10px] font-mono whitespace-pre-wrap break-all max-w-full">
                {formattedJson || displayResponse}
              </pre>
            ) : (
              <Suspense fallback={<span>{displayResponse}</span>}>
                <LazyReactMarkdown
                  components={{
                    p: ({ children }) => <p className="mb-2 last:mb-0 break-words">{children}</p>,
                    strong: ({ children }) => <strong className="font-bold">{children}</strong>,
                    em: ({ children }) => <em className="italic">{children}</em>,
                    ul: ({ children }) => <ul className="list-disc list-inside mb-2">{children}</ul>,
                    ol: ({ children }) => <ol className="list-decimal list-inside mb-2">{children}</ol>,
                    li: ({ children }) => <li className="mb-1">{children}</li>,
                    code: ({ children }) => (
                      <code className="bg-black/10 dark:bg-white/10 px-1 py-0.5 rounded text-xs font-mono break-words">
                        {children}
                      </code>
                    ),
                    pre: ({ children }) => (
                      <pre className="bg-black/5 dark:bg-black/20 p-2 rounded border border-current/10 overflow-x-auto text-xs font-mono my-2 whitespace-pre-wrap break-words max-w-full">
                        {children}
                      </pre>
                    ),
                    h1: ({ children }) => <h1 className="text-lg font-bold mb-2">{children}</h1>,
                    h2: ({ children }) => <h2 className="text-base font-bold mb-2">{children}</h2>,
                    h3: ({ children }) => <h3 className="text-sm font-bold mb-1">{children}</h3>,
                    hr: () => <hr className="my-3 border-current/20" />,
                  }}
                >
                  {displayResponse}
                </LazyReactMarkdown>
              </Suspense>
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
              <div className={cn("text-xs whitespace-pre-wrap break-words leading-relaxed pl-3 pb-4 border-l-2 border-current opacity-90", colors.text)}>
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
        <Suspense fallback={<div className="text-xs text-muted-foreground p-2">Loading diff...</div>}>
          <LazyDiffViewer
            diff={diff}
            className="max-w-[92%] md:max-w-[85%]"
          />
        </Suspense>
      )}
    </div>
  );
});

// Empty thinking state as a constant to avoid infinite loops
const EMPTY_THINKING_STATE = { content: "", isThinking: false } as const;

function exportSessionToMarkdown(messages: Message[], sessionId: string): string {
  const date = new Date().toISOString();
  let markdown = `# Session Export\n\n`;
  markdown += `**Session ID:** ${sessionId}\n`;
  markdown += `**Exported:** ${date}\n\n`;
  markdown += `---\n\n`;

  messages.forEach((message) => {
    const role = message.role === "user" ? "👤 User" : "🤖 Assistant";
    markdown += `## ${role}\n\n`;
    markdown += `**Time:** ${message.timestamp || "N/A"}\n\n`;
    markdown += `${message.content}\n\n`;
    markdown += `---\n\n`;
  });

  return markdown;
}

export function SessionHistory({ sessionId, className }: SessionHistoryProps) {
  // Session history component
  const needsAuth = useAppStore((state) => state.needsAuth);
  const {
    data: messages,
    isLoading,
    fetchOlder,
    hasOlder,
    isFetchingOlder,
  } = useSessionHistory({ sessionId, enabled: !needsAuth });
  const scrollRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const thinkingState = useThinkingStore((state) =>
    sessionId ? state.thinkingBySession[sessionId] ?? EMPTY_THINKING_STATE : EMPTY_THINKING_STATE
  );
  const prevSessionIdRef = useRef<string | null>(null);
  const initialScrollDoneRef = useRef<boolean>(false);
  const loadingOlderRef = useRef<boolean>(false);

  // Track previous message count to detect new messages
  const prevMessageCountRef = useRef<number>(0);

  // Throttled scroll ref to prevent excessive scroll calls
  const scrollTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isScrollingRef = useRef<boolean>(false);

  const handleExport = useCallback(() => {
    if (!messages || messages.length === 0) return;

    const markdown = exportSessionToMarkdown(messages, sessionId || "unknown");
    const blob = new Blob([markdown], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `session-${sessionId?.substring(0, 8)}-${new Date().toISOString().split("T")[0]}.md`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }, [messages, sessionId]);

  // Throttled auto-scroll to bottom when new messages arrive or thinking updates
  useEffect(() => {
    // If session ID changed, reset scroll flag
    if (sessionId !== prevSessionIdRef.current) {
      prevSessionIdRef.current = sessionId;
      initialScrollDoneRef.current = false;
      prevMessageCountRef.current = 0;
    }

    if (!messages) return;

    const messageCount = messages.length;
    if (loadingOlderRef.current) {
      prevMessageCountRef.current = messageCount;
      return;
    }
    const hasNewMessages = messageCount > prevMessageCountRef.current;

    // Only scroll if:
    // 1. Initial load (first time seeing messages)
    // 2. New messages added
    // 3. Thinking started (not on every delta)
    const shouldScroll = !initialScrollDoneRef.current ||
                         hasNewMessages ||
                         (thinkingState.isThinking && prevMessageCountRef.current === messageCount);

    if (shouldScroll && !isScrollingRef.current) {
      isScrollingRef.current = true;

      // Clear any pending scroll
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }

      // Use requestAnimationFrame + throttle to ensure DOM is updated
      scrollTimeoutRef.current = setTimeout(() => {
        requestAnimationFrame(() => {
          messagesEndRef.current?.scrollIntoView({
            behavior: initialScrollDoneRef.current ? "smooth" : "auto"
          });

          if (!initialScrollDoneRef.current && messages.length > 0) {
            initialScrollDoneRef.current = true;
          }

          // Allow next scroll after a delay
          setTimeout(() => {
            isScrollingRef.current = false;
          }, 100); // Throttle: max 10 scrolls/second
        });
      }, 16); // One frame delay
    }

    prevMessageCountRef.current = messageCount;

    return () => {
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
    };
  }, [messages, thinkingState.isThinking, sessionId]); // Note: using thinkingState.isThinking, not full object

  const handleLoadOlder = useCallback(async () => {
    if (!fetchOlder || isFetchingOlder) return;
    loadingOlderRef.current = true;
    try {
      await fetchOlder();
    } finally {
      loadingOlderRef.current = false;
    }
  }, [fetchOlder, isFetchingOlder]);

  if (!sessionId) {
    return (
      <div className={cn("flex items-center justify-center h-full", className)}>
        <div className="flex flex-col items-center gap-4 text-center px-6">
          <div className="rounded-full bg-muted p-4">
            <MessageSquare className="h-8 w-8 text-muted-foreground" />
          </div>
          <div className="space-y-1">
            <h3 className="text-lg font-semibold">No session selected</h3>
            <p className="text-sm text-muted-foreground max-w-[280px]">
              Open the command palette to switch sessions or create a new one.
            </p>
          </div>
          <kbd className="inline-flex items-center gap-1 rounded-lg border bg-muted px-3 py-1.5 font-mono text-sm text-muted-foreground">
            ⌘K
          </kbd>
        </div>
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

  // Warn about truncated sessions
  const isTruncated = Boolean(hasOlder);

  return (
    <div className={cn("flex flex-col h-full w-full max-w-full min-w-0 overflow-x-hidden", className)}>
      {messages && messages.length > 0 && (
        <div className="p-2 border-b hidden md:flex justify-between items-center bg-card">
          {isTruncated && (
            <div className="text-xs text-muted-foreground">
              Showing last {messages.length} messages
            </div>
          )}
          <div className={isTruncated ? "" : "ml-auto"}>
            <Button
              variant="outline"
              size="sm"
              onClick={handleExport}
              className="gap-2 rounded-wobblyMd border-2 shadow-hard-sm"
            >
              <Download className="h-4 w-4" />
              Export
            </Button>
          </div>
        </div>
      )}
      <ScrollArea className="flex-1 w-full max-w-full min-w-0 overflow-x-hidden">
        <div
          ref={scrollRef}
          className="w-full max-w-full min-w-0 box-border p-4 pb-6 md:pr-16 md:pb-24 space-y-6 overflow-x-hidden"
        >
          {hasOlder && (
            <div className="flex justify-center">
              <Button
                variant="ghost"
                size="sm"
                onClick={handleLoadOlder}
                disabled={isFetchingOlder}
                className="gap-2 rounded-lg text-muted-foreground"
              >
                {isFetchingOlder && <Loader2 className="h-4 w-4 animate-spin" />}
                Load older messages
              </Button>
            </div>
          )}
          {messages.map((message, index) => (
            <MessageBubble key={`${message.timestamp || ''}-${message.role}-${index}`} message={message} />
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
