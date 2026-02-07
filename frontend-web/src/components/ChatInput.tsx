import { useState, useRef, useEffect } from "react";
import { Button } from "./ui/button";
import { Textarea } from "./ui/textarea";
import { Send, Cpu, X, ImageIcon } from "lucide-react";
import { cn } from "../lib/utils";
import { usePiSettings, type PiModel } from "../hooks/usePiSettings";
import { useAppStore } from "../store/appStore";
import type { ImageUploadRequest } from "../types/api";

interface ChatInputProps {
  sessionId: string | null;
  onSendMessage: (content: string, images?: ImageUploadRequest[]) => void;
  disabled?: boolean;
  className?: string;
}

interface ImageWithPreview {
  file: File;
  preview: string | null;
}

const MAX_FILE_SIZE = 10 * 1024 * 1024;
const ALLOWED_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];

export function ChatInput({
  sessionId,
  onSendMessage,
  disabled = false,
  className,
}: ChatInputProps) {
  const needsAuth = useAppStore((state) => state.needsAuth);
  const [content, setContent] = useState("");
  const [selectedImages, setSelectedImages] = useState<ImageWithPreview[]>([]);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { data: settings, isLoading: settingsLoading } = usePiSettings(!needsAuth);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
    }
  }, [content]);

  const validateFile = (file: File): string | null => {
    if (!ALLOWED_TYPES.includes(file.type)) {
      return `Invalid file type: ${file.type}. Only PNG, JPEG, GIF, and WebP are allowed.`;
    }
    if (file.size > MAX_FILE_SIZE) {
      return `File too large: ${(file.size / 1024 / 1024).toFixed(2)}MB. Maximum is 10MB.`;
    }
    return null;
  };

  const fileToPreview = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
  };

  const addImages = async (files: File[]) => {
    const imagesWithPreviews = await Promise.all(
      files.map(async (file) => ({
        file,
        preview: await fileToPreview(file),
      }))
    );
    setSelectedImages((prev) => [...prev, ...imagesWithPreviews]);
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    const validFiles: File[] = [];

    for (const file of files) {
      const error = validateFile(file);
      if (error) {
        alert(error);
        continue;
      }
      validFiles.push(file);
    }

    if (validFiles.length > 0) {
      addImages(validFiles);
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    const imageItems = Array.from(items).filter((item) =>
      item.type.startsWith("image/")
    );

    if (imageItems.length > 0) {
      e.preventDefault();
      const files = imageItems
        .map((item) => item.getAsFile())
        .filter((file): file is File => {
          if (!file) return false;
          const error = validateFile(file);
          if (error) {
            alert(error);
            return false;
          }
          return true;
        });

      if (files.length > 0) {
        addImages(files);
      }
    }
  };

  const removeImage = (index: number) => {
    setSelectedImages((prev) => prev.filter((_, i) => i !== index));
  };

  const fileToBase64 = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        resolve(result.split(",")[1]);
      };
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
  };

  const isDisabled =
    disabled ||
    !sessionId ||
    (content.trim().length === 0 && selectedImages.length === 0);

  const handleSend = async () => {
    if (isDisabled) return;

    const trimmed = content.trim();
    const imageRequests: ImageUploadRequest[] = await Promise.all(
      selectedImages.map(async ({ file }) => ({
        filename: file.name,
        content_type: file.type,
        data: await fileToBase64(file),
      }))
    );

    if (trimmed || imageRequests.length > 0) {
      onSendMessage(trimmed, imageRequests);
      setContent("");
      setSelectedImages([]);

      if (textareaRef.current) {
        textareaRef.current.style.height = "auto";
      }

      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (!isDisabled) {
        handleSend();
      }
    }
  };

  const currentModel = settings?.availableModels?.find(
    (model: PiModel) => model.id === settings?.defaultModel
  );
  const modelDisplay = currentModel?.name || settings?.defaultModel || "Not configured";
  const providerDisplay = currentModel?.provider;

  return (
    <div className={cn("border-t bg-background p-3", className)}>
      {selectedImages.length > 0 && (
        <div className="flex flex-wrap gap-2 mb-3 max-w-4xl mx-auto">
          {selectedImages.map((img, index) => (
            <div key={index} className="relative group">
              <img
                src={img.preview || undefined}
                alt={`Preview ${index + 1}`}
                className="h-20 w-20 object-cover rounded-lg border border-border"
              />
              <button
                onClick={() => removeImage(index)}
                className="absolute -top-2 -right-2 bg-red-500 text-white rounded-full p-1
                         opacity-0 group-hover:opacity-100 transition-opacity shadow-sm"
                type="button"
              >
                <X size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      <div className="flex items-center gap-2 max-w-4xl mx-auto px-2">
        <Button
          onClick={() => fileInputRef.current?.click()}
          disabled={!sessionId || disabled}
          size="icon"
          variant="outline"
          className="h-[44px] w-[44px] shrink-0"
          type="button"
          title="Attach image (or paste from clipboard)"
        >
          <ImageIcon className="h-4 w-4" />
        </Button>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/png,image/jpeg,image/gif,image/webp"
          multiple
          className="hidden"
          onChange={handleFileSelect}
        />

        <Textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder={
            !sessionId
              ? "Select a session"
              : "Type a message... (Shift+Enter for new line, paste images or click 📎)"
          }
          disabled={!sessionId || disabled}
          className="min-h-[44px] max-h-[200px] resize-none text-base py-2.5 leading-6 flex-1"
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
          type="button"
        >
          <Send className="h-4 w-4" />
        </Button>
      </div>

      {sessionId && (
        <div className="flex items-center gap-1.5 max-w-4xl mx-auto mt-2 px-2">
          <Cpu className="h-3 w-3 text-muted-foreground shrink-0" />
          <span className="text-xs text-muted-foreground hidden sm:inline">
            Model:
          </span>
          <span className="text-xs text-muted-foreground font-medium truncate max-w-[200px] sm:max-w-[300px]">
            {settingsLoading ? "Loading..." : modelDisplay}
          </span>
          {providerDisplay && (
            <span className="text-xs text-muted-foreground/70 hidden md:inline">
              ({providerDisplay})
            </span>
          )}
        </div>
      )}
    </div>
  );
}
