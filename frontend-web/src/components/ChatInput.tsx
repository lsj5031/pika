import { useState, useRef, useEffect } from "react";
import { Button } from "./ui/button";
import { Textarea } from "./ui/textarea";
import { Send } from "lucide-react";
import { cn } from "../lib/utils";

interface ChatInputProps {
  sessionId: string | null;
  isSessionActive: boolean;
  onSendMessage: (content: string) => void;
  disabled?: boolean;
  className?: string;
}

export function ChatInput({
  sessionId,
  isSessionActive,
  onSendMessage,
  disabled = false,
  className,
}: ChatInputProps) {
  const [content, setContent] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-resize textarea based on content
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
    }
  }, [content]);

  // Check if send button should be disabled
  const isDisabled =
    disabled ||
    !sessionId ||
    content.trim().length === 0;
  // Removed: !isSessionActive - chat should work even if session isn't active

  const handleSend = () => {
    const trimmed = content.trim();
    if (trimmed && !isDisabled) {
      onSendMessage(trimmed);
      setContent("");
      // Reset textarea height
      if (textareaRef.current) {
        textareaRef.current.style.height = "auto";
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className={cn("border-t bg-background p-4", className)}>
      <div className="flex items-end gap-2 max-w-4xl mx-auto">
        <Textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={
            !sessionId
              ? "Select a session"
              : "Type a message... (Shift+Enter for new line)"
          }
          disabled={!sessionId || disabled}
          // Removed: !isSessionActive check
          className="min-h-[44px] max-h-[200px] resize-none"
          rows={1}
          id="chat-input"
          data-testid="chat-input"
          enterKeyHint="send"
        />
        <Button
          onClick={handleSend}
          disabled={isDisabled}
          size="icon"
          className="h-[44px] w-[44px] shrink-0"
          id="send-button"
          data-testid="send-button"
        >
          <Send className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
