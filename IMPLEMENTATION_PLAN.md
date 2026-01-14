# PI Agent Manager - Implementation Plan

**Date:** 2025-01-14
**Issues:** 7 frontend/backend bugs and features to implement

---

## Overview

This plan addresses 7 critical issues:
1. Chat box disabled - should be active in all sessions
2. Empty chat messages from tool call results not parsing
3. Code diff integration with diffs.com
4. Session in-progress notifications/icons
5. Unread/finished session indicators
6. Mobile session list not working
7. Login modal grey/invisible

---

## ISSUE 1: Chat Box Disabled - Should be Active in All Sessions

**Root Cause:** The `ChatInput` component disables input when `isSessionActive` is false, but sessions should allow chat even when not actively running.

### Files to Modify
- `frontend-web/src/components/ChatInput.tsx`

### Changes

**Location: Lines 17-30**
```typescript
// BEFORE:
  const isDisabled =
    disabled ||
    !sessionId ||
    !isSessionActive ||
    content.trim().length === 0;

// AFTER:
  const isDisabled =
    disabled ||
    !sessionId ||
    content.trim().length === 0;
  // Removed: !isSessionActive - chat should work even if session isn't active
```

**Location: Lines 60-72**
```typescript
// BEFORE:
        <Textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={
            !sessionId
              ? "Select a session"
              : !isSessionActive
                ? "Session is not active"
                : "Type a message... (Shift+Enter for new line)"
          }
          disabled={!sessionId || !isSessionActive || disabled}
          className="min-h-[44px] max-h-[200px] resize-none"
          rows={1}
          id="chat-input"
          data-testid="chat-input"
          enterKeyHint="send"
        />

// AFTER:
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
```

### Testing
1. Select an inactive session
2. Chat input should be enabled (not greyed out)
3. Send a message - should be delivered successfully

---

## ISSUE 2: Empty Chat Messages from Tool Call Results Not Parsing

**Root Cause:** The message parsing in `sessions.rs` filters out entries without proper content structure. Tool call results may have empty or malformed content arrays.

### Files to Modify
- `src/sessions.rs` (lines 252-268)

### Changes

**Location: Lines 252-268 in src/sessions.rs**
```rust
// BEFORE:
            // Get content from message.content array
            let content = if let Some(content_array) = message_obj.get("content").and_then(|c| c.as_array()) {
                // Concatenate all text parts
                content_array
                    .iter()
                    .filter_map(|part| {
                        part.get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                continue;
            };

// AFTER:
            // Get content from message.content array
            // Handle both text content and tool call results
            let content = if let Some(content_array) = message_obj.get("content").and_then(|c| c.as_array()) {
                // Try to get text parts first
                let text_parts: Vec<String> = content_array
                    .iter()
                    .filter_map(|part| {
                        part.get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();

                if !text_parts.is_empty() {
                    text_parts.join("\n")
                } else {
                    // Try to extract tool call information
                    let tool_parts: Vec<String> = content_array
                        .iter()
                        .filter_map(|part| {
                            // Handle tool_use type
                            if let Some(tool_use) = part.get("tool_use").and_then(|t| t.as_object()) {
                                let name = tool_use.get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown_tool");
                                let input = tool_use.get("input")
                                    .and_then(|i| {
                                        if i.is_string() {
                                            i.as_str()
                                        } else if i.is_object() {
                                            Some(serde_json::to_string(i).unwrap_or_default())
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or("");
                                Some(format!("Tool Call: {}({})", name, input))
                            }
                            // Handle tool_result type
                            else if let Some(tool_result) = part.get("tool_result").and_then(|t| t.as_object()) {
                                let is_error = tool_result.get("is_error")
                                    .and_then(|e| e.as_bool())
                                    .unwrap_or(false);
                                let content = tool_result.get("content")
                                    .and_then(|c| {
                                        if c.is_string() {
                                            c.as_str()
                                        } else if c.is_array() {
                                            serde_json::to_string(c).ok()
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or("");
                                Some(format!("Tool Result{}: {}",
                                    if is_error { " (Error)" } else { "" },
                                    content
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !tool_parts.is_empty() {
                        tool_parts.join("\n")
                    } else {
                        // Fallback: show entire content array as JSON for debugging
                        format!("Tool call: {}", serde_json::to_string(content_array).unwrap_or_default())
                    }
                }
            } else if let Some(content_str) = message_obj.get("content").and_then(|c| c.as_str()) {
                // Handle string content directly
                content_str.to_string()
            } else {
                // Empty or unparseable content - don't skip, show placeholder
                String::from("[Tool call or system message - no text content]")
            };
```

### Testing
1. Send a message that triggers a tool call (e.g., "list files")
2. Verify the tool call result appears in the chat
3. Check console for any skipped message warnings

---

## ISSUE 3: Code Diff Integration with diffs.com

### Files to Create
- `frontend-web/src/components/DiffViewer.tsx` (NEW)
- `frontend-web/src/types/api.ts` (add types)

### Files to Modify
- `frontend-web/src/components/SessionHistory.tsx`
- `frontend-web/src/index.css`

### Implementation

**NEW FILE: `frontend-web/src/components/DiffViewer.tsx`**
```typescript
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
```

**MODIFY: `frontend-web/src/types/api.ts`**
```typescript
// ADD to existing types:
export interface CodeDiff {
  id: string;
  sessionId: string;
  messageId: string;
  oldCode: string;
  newCode: string;
  language: string;
  filePath: string;
  createdAt: string;
  diffUrl?: string;
}

export interface DiffIntegrationSettings {
  enabled: boolean;
  apiKey?: string;
  autoGenerate: boolean;
}
```

**MODIFY: `frontend-web/src/components/SessionHistory.tsx`**
```typescript
// ADD import:
import { DiffViewer } from "./DiffViewer";

// ADD diff parsing function:
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

// MODIFY MessageBubble component - ADD diff viewer:
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
```

### Testing
1. Send a message that results in code changes
2. Verify the DiffViewer component appears below the message
3. Test split/unified view toggle
4. Verify file path and language badges show correctly

---

## ISSUE 4: Session In-Progress Notifications/Icons

### Files to Modify
- `frontend-web/src/store/appStore.ts`
- `frontend-web/src/components/SessionList.tsx`
- `frontend-web/src/App.tsx`

### Implementation

**MODIFY: `frontend-web/src/store/appStore.ts`**
```typescript
// ADD to AppState interface:
interface AppState {
  // State
  currentSessionId: string | null;
  sidebarCollapsed: boolean;
  activeSessionIds: Set<string>; // NEW: Track active sessions
  thinkingSessionIds: Set<string>; // NEW: Track sessions with thinking in progress

  // Actions
  setCurrentSession: (sessionId: string | null) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActiveSession: (sessionId: string, isActive: boolean) => void; // NEW
  setThinkingSession: (sessionId: string, isThinking: boolean) => void; // NEW
}

// UPDATE store implementation:
export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Initial state
      currentSessionId: null,
      sidebarCollapsed: false,
      activeSessionIds: new Set<string>(),
      thinkingSessionIds: new Set<string>(),

      // Actions
      setCurrentSession: (sessionId) => set({ currentSessionId: sessionId }),

      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

      setActiveSession: (sessionId, isActive) =>
        set((state) => {
          const newSet = new Set(state.activeSessionIds);
          if (isActive) {
            newSet.add(sessionId);
          } else {
            newSet.delete(sessionId);
          }
          return { activeSessionIds: newSet };
        }),

      setThinkingSession: (sessionId, isThinking) =>
        set((state) => {
          const newSet = new Set(state.thinkingSessionIds);
          if (isThinking) {
            newSet.add(sessionId);
          } else {
            newSet.delete(sessionId);
          }
          return { thinkingSessionIds: newSet };
        }),
    }),
    {
      name: "pi-agent-manager-storage",
      // Don't persist Sets (they'll be re-synced from WebSocket)
      partialize: (state) => ({
        currentSessionId: state.currentSessionId,
        sidebarCollapsed: state.sidebarCollapsed,
      }),
    }
  )
);
```

**MODIFY: `frontend-web/src/components/SessionList.tsx`**
```typescript
// ADD import:
import { Loader2 } from "lucide-react";

// ADD to component hooks:
export function SessionList({ className }: SessionListProps) {
  // ... existing code ...
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const activeSessionIds = useAppStore((state) => state.activeSessionIds);
  const thinkingSessionIds = useAppStore((state) => state.thinkingSessionIds);
  // ... existing code ...
```

```typescript
// REPLACE the session button content (around lines 67-86):
                        <button
                          onClick={() => handleSessionSelect(session.id)}
                          className={cn(
                            "w-full flex items-center gap-2 px-3 py-3 text-sm rounded-wobbly transition-all min-h-[44px]",
                            "hover:bg-accent hover:text-accent-foreground hover:rotate-1",
                            currentSessionId === session.id &&
                            "bg-accent text-accent-foreground rotate-1 shadow-sm"
                          )}
                          data-testid={`session-item-${session.id}`}
                        >
                          {/* Status indicator with multiple states */}
                          <div className="relative">
                            {/* Active/inactive dot */}
                            {(activeSessionIds.has(session.id) || session.is_active) && (
                              <span
                                className="h-2 w-2 rounded-full bg-green-500"
                                aria-label="Active session"
                              />
                            )}
                            {/* Thinking spinner overlay */}
                            {thinkingSessionIds.has(session.id) && (
                              <span className="absolute -top-1 -right-1">
                                <Loader2 className="h-3 w-3 animate-spin text-blue-500" />
                              </span>
                            )}
                            {/* Empty placeholder for spacing */}
                            {!activeSessionIds.has(session.id) &&
                             !session.is_active &&
                             !thinkingSessionIds.has(session.id) && (
                              <span className="h-2 w-2" aria-hidden="true" />
                            )}
                          </div>

                          {/* Session name */}
                          <span className="flex-1 text-left truncate">
                            {session.name || "Untitled Session"}
                          </span>
                        </button>
```

**MODIFY: `frontend-web/src/App.tsx` - Update WebSocket handler**
```typescript
// UPDATE handleWebSocketMessage callback:
  const handleWebSocketMessage = useCallback(
    (event: WSEvent) => {
      switch (event.type) {
        case "SessionStarted": {
          // Update session active status
          useAppStore.getState().setActiveSession(event.data.session_id, true);
          queryClient.invalidateQueries({ queryKey: ["sessions"] });
          break;
        }
        case "SessionStopped": {
          // Clear thinking state and active status
          clearThinking(event.data.session_id);
          useAppStore.getState().setActiveSession(event.data.session_id, false);
          queryClient.invalidateQueries({ queryKey: ["sessions"] });
          break;
        }
        case "ThinkingDelta": {
          // Set thinking state
          useAppStore.getState().setThinkingSession(event.data.session_id, true);
          appendThinking(event.data.session_id, event.data.content);
          break;
        }
        case "MessageAdded": {
          // Clear thinking state when message is added
          useAppStore.getState().setThinkingSession(event.data.session_id, false);
          clearThinking(event.data.session_id);
          queryClient.invalidateQueries({
            queryKey: ["sessions", event.data.session_id, "messages"],
          });
          break;
        }
      }
    },
    [queryClient, appendThinking, clearThinking]
  );
```

### Testing
1. Start an agent session
2. Verify green dot appears next to session name
3. When agent is thinking, verify blue spinner appears
4. When thinking stops, verify spinner disappears
5. Stop session - verify green dot disappears

---

## ISSUE 5: Unread/Finished Session Indicators

### Files to Modify
- `frontend-web/src/store/appStore.ts`
- `frontend-web/src/components/SessionList.tsx`
- `frontend-web/src/App.tsx`

### Implementation

**MODIFY: `frontend-web/src/store/appStore.ts`**
```typescript
// ADD to AppState interface:
interface AppState {
  // ... existing ...
  unreadSessions: Set<string>; // NEW: Track sessions with unread messages
  lastSeenMessageCounts: Record<string, number>; // NEW: Track last seen message count per session

  // Actions
  // ... existing ...
  markSessionAsRead: (sessionId: string, messageCount: number) => void; // NEW
  incrementUnreadCount: (sessionId: string) => void; // NEW
}

// UPDATE implementation:
export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Initial state
      currentSessionId: null,
      sidebarCollapsed: false,
      activeSessionIds: new Set<string>(),
      thinkingSessionIds: new Set<string>(),
      unreadSessions: new Set<string>(),
      lastSeenMessageCounts: {},

      // ... existing actions ...

      markSessionAsRead: (sessionId, messageCount) =>
        set((state) => {
          const newUnread = new Set(state.unreadSessions);
          newUnread.delete(sessionId);
          return {
            unreadSessions: newUnread,
            lastSeenMessageCounts: {
              ...state.lastSeenMessageCounts,
              [sessionId]: messageCount,
            },
          };
        }),

      incrementUnreadCount: (sessionId) =>
        set((state) => {
          // Only mark as unread if it's not the current session
          if (state.currentSessionId === sessionId) {
            return state;
          }
          const newUnread = new Set(state.unreadSessions);
          newUnread.add(sessionId);
          return { unreadSessions: newUnread };
        }),
    }),
    {
      name: "pi-agent-manager-storage",
      partialize: (state) => ({
        currentSessionId: state.currentSessionId,
        sidebarCollapsed: state.sidebarCollapsed,
        lastSeenMessageCounts: state.lastSeenMessageCounts,
      }),
    }
  )
);
```

**MODIFY: `frontend-web/src/components/SessionList.tsx`**
```typescript
// ADD import:
import { Badge } from "./ui/badge";

// ADD to component hooks:
export function SessionList({ className }: SessionListProps) {
  // ... existing ...
  const unreadSessions = useAppStore((state) => state.unreadSessions);
  // ... existing ...
```

```typescript
// REPLACE session name section with:
                          {/* Session name with unread indicator */}
                          <div className="flex-1 flex items-center gap-2">
                            <span className="flex-1 text-left truncate">
                              {session.name || "Untitled Session"}
                            </span>

                            {/* Unread badge */}
                            {unreadSessions.has(session.id) && (
                              <Badge
                                variant="default"
                                className="h-5 px-1.5 text-xs bg-accent text-accent-foreground"
                              >
                                •
                              </Badge>
                            )}
                          </div>
```

**MODIFY: `frontend-web/src/App.tsx`**
```typescript
// ADD to component hooks:
  const markSessionAsRead = useAppStore((state) => state.markSessionAsRead);

// ADD effect for tracking current session:
  useEffect(() => {
    if (currentSessionId) {
      // Mark current session as read when selected
      const session = sessions?.find((s) => s.id === currentSessionId);
      if (session) {
        // Get current message count
        queryClient.fetchQuery({
          queryKey: ["sessions", currentSessionId, "messages"],
        }).then((messages: any) => {
          markSessionAsRead(currentSessionId, messages?.length || 0);
        });
      }
    }
  }, [currentSessionId, sessions, queryClient, markSessionAsRead]);

// UPDATE WebSocket handler - MessageAdded case:
    case "MessageAdded": {
      // Mark as unread if not current session
      if (currentSessionId !== event.data.session_id) {
        useAppStore.getState().incrementUnreadCount(event.data.session_id);
      }
      useAppStore.getState().setThinkingSession(event.data.session_id, false);
      clearThinking(event.data.session_id);
      queryClient.invalidateQueries({
        queryKey: ["sessions", event.data.session_id, "messages"],
      });
      break;
    }
```

### Testing
1. Have session A open
2. Receive a message in session B
3. Verify badge (•) appears next to session B
4. Click on session B
5. Verify badge disappears

---

## ISSUE 6: Mobile Session List Not Working

### Files to Modify
- `frontend-web/src/components/ui/sheet.tsx`
- `frontend-web/src/App.tsx`
- `frontend-web/src/AppHeader.tsx`
- `frontend-web/src/index.css`

### Implementation

**MODIFY: `frontend-web/src/components/ui/sheet.tsx`**
```typescript
// UPDATE SheetContent className (around line 60):
const SheetContent = React.forwardRef<
  React.ElementRef<typeof SheetPrimitive.Content>,
  SheetContentProps
>(({ side = "right", className, children, ...props }, ref) => (
  <SheetPortal>
    <SheetOverlay className="z-[60]" /> {/* Add explicit z-index */}
    <SheetPrimitive.Content
      ref={ref}
      className={cn(
        sheetVariants({ side }),
        "z-[70]", // Add explicit z-index above overlay
        className
      )}
      style={{ touchAction: "auto" }} // Ensure touch works
      {...props}
    >
      {children}
      <SheetPrimitive.Close
        className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-secondary z-[80]" // Add z-index
        style={{ touchAction: "manipulation" }} // Ensure close button works
      >
        <X className="h-4 w-4" />
        <span className="sr-only">Close</span>
      </SheetPrimitive.Close>
    </SheetPrimitive.Content>
  </SheetPortal>
))
```

**MODIFY: `frontend-web/src/App.tsx`**
```typescript
// UPDATE Sheet component (around lines 171-176):
          {/* Mobile Drawer Sidebar */}
          <Sheet open={mobileDrawerOpen} onOpenChange={setMobileDrawerOpen}>
            <SheetContent
              side="left"
              className="w-[85vw] max-w-[320px] p-0 sm:w-64"
              id="mobile-drawer-content"
              style={{
                touchAction: "auto",
                WebkitOverflowScrolling: "touch",
              }}
            >
              <div className="h-full overflow-y-auto">
                <SessionList />
              </div>
            </SheetContent>
          </Sheet>
```

**MODIFY: `frontend-web/src/AppHeader.tsx`**
```typescript
// UPDATE menu button:
        <Button
          variant="ghost"
          size="icon"
          className="md:hidden min-w-[44px] min-h-[44px] touch-manipulation"
          onClick={onMenuToggle}
          id="session-list-button"
          data-testid="session-list-button"
          style={{ touchAction: "manipulation" }}
        >
          <Menu className="h-5 w-5 pointer-events-none" />
          <span className="sr-only">Toggle menu</span>
        </Button>
```

**MODIFY: `frontend-web/src/index.css` - ADD at end of file**
```css
@layer components {
  /* Mobile sheet/session list fixes */
  [data-radix-scroll-area-viewport] {
    touch-action: pan-y;
    -webkit-overflow-scrolling: touch;
  }

  /* Ensure buttons in sheets are clickable */
  [role="dialog"] button,
  [role="dialog"] [role="button"] {
    touch-action: manipulation;
    cursor: pointer;
    -webkit-tap-highlight-color: rgba(255, 77, 77, 0.2);
  }

  /* Fix sheet close button on mobile */
  [data-radix-dialog-close] {
    min-width: 44px !important;
    min-height: 44px !important;
  }

  /* Prevent body scroll when sheet is open */
  body[data-radix-dialog-open] {
    overflow: hidden !important;
    position: fixed;
    width: 100%;
  }
}
```

### Testing
1. Open Chrome DevTools
2. Enable mobile device emulation (iPhone 12 or similar)
3. Click hamburger menu
4. Verify drawer opens from left
5. Verify session list is visible and scrollable
6. Click on a session - verify it works
7. Click close button - verify drawer closes
8. Click outside drawer - verify it closes

---

## ISSUE 7: Login Modal Grey/Invisible

### Files to Modify
- `frontend-web/src/components/ui/dialog.tsx`
- `frontend-web/src/components/AuthPrompt.tsx`
- `frontend-web/src/index.css`

### Implementation

**MODIFY: `frontend-web/src/components/ui/dialog.tsx`**
```typescript
// UPDATE DialogOverlay (lines 23-31):
const DialogOverlay = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Overlay>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay
    ref={ref}
    className={cn(
      "fixed inset-0 z-50 bg-black/60 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0", // Changed from bg-black/80 to bg-black/60
      className
    )}
    {...props}
  />
))

// UPDATE DialogContent (lines 33-52):
const DialogContent = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content>
>(({ className, children, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      className={cn(
        "fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border-2 border-primary bg-white p-6 shadow-hard duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] rounded-wobblyMd font-body text-foreground",
        // Changed: bg-background -> bg-white, added text-foreground
        className
      )}
      {...props}
    >
      {children}
      <DialogPrimitive.Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground z-[100]">
        <X className="h-4 w-4" />
        <span className="sr-only">Close</span>
      </DialogPrimitive.Close>
    </DialogPrimitive.Content>
  </DialogPortal>
))
```

**MODIFY: `frontend-web/src/components/AuthPrompt.tsx`**
```typescript
// UPDATE DialogContent (around lines 46-56):
            <DialogContent
                className="sm:max-w-md bg-white shadow-2xl"
                onInteractOutside={(e) => e.preventDefault()}
                onEscapeKeyDown={(e) => e.preventDefault()}
            >
                <DialogHeader className="text-foreground">
                    <DialogTitle className="flex items-center gap-2 text-xl">
                        <Lock className="h-5 w-5" />
                        <span>Authentication Required</span>
                    </DialogTitle>
                    <DialogDescription className="text-muted-foreground">
                        Please enter your credentials to access PI Agent Manager.
                    </DialogDescription>
                </DialogHeader>

                <div className="grid gap-4 py-4">
                    <div className="grid gap-2">
                        <Label htmlFor="auth-username" className="text-foreground">Username</Label>
                        <Input
                            id="auth-username"
                            type="text"
                            placeholder="Enter username"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            onKeyDown={handleKeyDown}
                            autoComplete="username"
                            autoFocus
                            className="bg-white border-input"
                        />
                    </div>

                    <div className="grid gap-2">
                        <Label htmlFor="auth-password" className="text-foreground">Password</Label>
                        <Input
                            id="auth-password"
                            type="password"
                            placeholder="Enter password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            onKeyDown={handleKeyDown}
                            autoComplete="current-password"
                            className="bg-white border-input"
                        />
                    </div>

                    {error && (
                        <p className="text-sm text-destructive font-medium bg-destructive/10 p-2 rounded">{error}</p>
                    )}
                </div>

                <DialogFooter>
                    <Button
                        onClick={handleSubmit}
                        disabled={isLoading || !username.trim() || !password.trim()}
                        className="w-full"
                        id="auth-submit-button"
                    >
                        {isLoading ? "Authenticating..." : "Sign In"}
                    </Button>
                </DialogFooter>
            </DialogContent>
```

**MODIFY: `frontend-web/src/index.css` - ADD at end of file**
```css
@layer components {
  /* Dialog/Modal visibility fixes */
  [data-radix-dialog-content] {
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
  }

  /* Ensure text is visible in dialogs */
  [role="dialog"],
  [data-radix-dialog-content] {
    color: var(--foreground);
  }

  [role="dialog"] h1,
  [role="dialog"] h2,
  [role="dialog"] h3,
  [role="dialog"] label {
    color: var(--foreground);
  }

  /* Input visibility in dialogs */
  [role="dialog"] input,
  [role="dialog"] textarea {
    background-color: white !important;
    color: var(--foreground) !important;
    border-color: var(--border) !important;
  }

  /* Dark mode support for dialogs */
  .dark [role="dialog"] input,
  .dark [role="dialog"] textarea {
    background-color: var(--background) !important;
    color: var(--foreground) !important;
  }

  /* Dialog overlay should darken but not too much */
  [data-radix-dialog-overlay] {
    backdrop-filter: blur(2px);
  }
}
```

### Testing
1. Logout of the application
2. Verify login modal appears with white background
3. Verify all text is visible (black on white)
4. Verify input fields have white background
5. Verify submit button is visible and clickable
6. Test in both light and dark mode

---

## Summary

### Files to Modify (13 files)
1. `frontend-web/src/components/ChatInput.tsx` - Remove isSessionActive check
2. `src/sessions.rs` - Enhanced message parsing for tool calls
3. `frontend-web/src/components/DiffViewer.tsx` - NEW FILE
4. `frontend-web/src/components/SessionHistory.tsx` - Add diff detection
5. `frontend-web/src/store/appStore.ts` - Add active/thinking/unread tracking
6. `frontend-web/src/components/SessionList.tsx` - Status indicators, unread badges
7. `frontend-web/src/App.tsx` - WebSocket handlers, mobile sheet fixes
8. `frontend-web/src/components/ui/sheet.tsx` - Z-index fixes
9. `frontend-web/src/components/ui/dialog.tsx` - Visibility fixes
10. `frontend-web/src/components/AuthPrompt.tsx` - Styling fixes
11. `frontend-web/src/AppHeader.tsx` - Mobile button fix
12. `frontend-web/src/index.css` - Mobile and dialog CSS fixes
13. `frontend-web/src/types/api.ts` - New types for diffs

### Priority Order
1. **P1 (Critical):** Issues #1, #2, #7 - Chat input, tool parsing, login modal
2. **P2 (UX):** Issues #4, #5, #6 - Status indicators, unread badges, mobile
3. **P3 (Feature):** Issue #3 - Diff integration

### Dependencies
- #1 (chat input) is independent
- #2 (message parsing) is independent (backend only)
- #7 (modal z-index) is independent
- #6 (mobile drawer) must work before #4 and #5 (indicators need to be visible)
- #4 (status) and #5 (unread) can be done in parallel
- #3 (diffs.com) is independent but requires understanding message structure from #2

### Testing Checklist
- [ ] Chat input works when session inactive
- [ ] Tool call results appear in chat
- [ ] Diff viewer shows for code changes
- [ ] Session status indicators (green dot, spinner) work
- [ ] Unread badges appear and disappear correctly
- [ ] Mobile session list opens and works
- [ ] Login modal is visible with white background
