import { toast } from "sonner";

export function showError(title: string, error: unknown) {
  let message = "An unknown error occurred";

  if (error instanceof Error) {
    message = error.message;
  } else if (typeof error === "string") {
    message = error;
  } else if (error && typeof error === "object" && "message" in error) {
    message = String(error.message);
  }

  toast.error(title, {
    description: message,
  });
}

export function showSuccess(title: string, description?: string) {
  toast.success(title, {
    description,
  });
}

export function showInfo(title: string, description?: string) {
  toast(title, {
    description,
  });
}
