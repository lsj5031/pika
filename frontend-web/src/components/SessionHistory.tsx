import { useEffect, useRef } from "react";
import { useSessionHistory } from "../hooks/useSessionHistory";
import { useThinkingStore } from "../store/thinkingStore";
import { Card } from "./ui/card";
import { ScrollArea } from "./ui/scroll-area";
import { ThinkingIndicator } from "./ThinkingIndicator";
import { DiffViewer } from "./DiffViewer";
import { cn } from "../lib/utils";
import type { Message } from "../types";
import { Bot, User, Wrench } from "lucide-react";

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
  // Look for code blocks with file paths
  const fileBlockRegex = /```(\w+)?\n(?:\/\/ (.+?)\n)?([\s\S]*?)```/g;
  const matches = [...content.matchAll(fileBlockRegex)];

  if (matches.length >= 2) {
    return {
      filePath: matches[0][2] || undefined,
      language: matches[0][1] || "text",
      oldContent: matches[0][3]?.trim(),
      newContent: matches[1][3]?.trim(),
    };
  }
  return null;
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
  const isUser = message.role === "user";
  const diff = !isUser ? parseDiffFromMessage(message.content) : null;
  const { thinking, response } = !isUser ? parseThinkingBlocks(message.content) : { thinking: "", response: message.content };
  const hasToolUse = message.content.includes("tool_use") || message.content.includes("Tool Call");
  const colors = getMessageColors(message.role, hasToolUse);
  const showThinking = thinking && thinking.length > 0;

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
          "max-w-[85%] px-4 py-3 border-2 shadow-sm transition-all hover:shadow-md",
          colors.bg,
          colors.border
        )}
      >
        {/* Response content */}
        {response && (
          <p className={cn("text-sm whitespace-pre-wrap break-words leading-relaxed", colors.text)}>
            {response}
          </p>
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
          className="max-w-[85%]"
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

  // Auto-scroll to bottom when new messages arrive or thinking updates
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, thinkingState]);

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
