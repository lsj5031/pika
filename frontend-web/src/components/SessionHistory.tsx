import { useEffect, useRef } from "react";
import { useSessionHistory } from "../hooks/useSessionHistory";
import { useThinkingStore } from "../store/thinkingStore";
import { Card } from "./ui/card";
import { ScrollArea } from "./ui/scroll-area";
import { ThinkingIndicator } from "./ThinkingIndicator";
import { DiffViewer } from "./DiffViewer";
import { cn } from "../lib/utils";
import type { Message } from "../types";

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

function MessageBubble({ message }: { message: Message }) {
  const isUser = message.role === "user";
  const diff = !isUser ? parseDiffFromMessage(message.content) : null;

  return (
    <div
      className={cn(
        "flex w-full flex-col gap-2",
        isUser ? "items-end" : "items-start"
      )}
    >
      <Card
        className={cn(
          "max-w-[80%] px-4 py-2",
          isUser
            ? "bg-primary text-primary-foreground"
            : "bg-muted"
        )}
      >
        <p className="text-sm whitespace-pre-wrap break-words">
          {message.content}
        </p>
        {message.timestamp && (
          <p
            className={cn(
              "text-xs mt-1",
              isUser
                ? "text-primary-foreground/70"
                : "text-muted-foreground"
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
          className="max-w-[80%]"
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
        <div ref={scrollRef} className="p-4 space-y-4">
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
