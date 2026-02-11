import { useState, useCallback } from "react";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Lock } from "lucide-react";
import { markAuthenticated } from "../lib/auth";
import { config } from "../config/env";

interface AuthPromptProps {
    open: boolean;
    onAuthenticated: () => void;
}

export function AuthPrompt({ open, onAuthenticated }: AuthPromptProps) {
    const [username, setUsername] = useState("");
    const [password, setPassword] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);

    const handleSubmit = useCallback(async () => {
        if (!username.trim() || !password.trim()) {
            setError("Please enter both username and password");
            return;
        }

        setIsLoading(true);
        setError(null);

        try {
            const response = await fetch(`${config.API_URL}/api/auth/login`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                credentials: "include",
                body: JSON.stringify({
                    username: username.trim(),
                    password,
                }),
            });

            if (response.ok) {
                markAuthenticated();
                onAuthenticated();
            } else if (response.status === 401) {
                setError("Invalid username or password");
            } else if (response.status === 429) {
                setError("Too many attempts. Please try again shortly.");
            } else {
                setError(`Authentication failed: ${response.statusText}`);
            }
        } catch {
            setError("Unable to reach server. Check your connection and try again.");
        } finally {
            setIsLoading(false);
        }
    }, [username, password, onAuthenticated]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !isLoading) {
            handleSubmit();
        }
    };

    return (
        <Dialog open={open} onOpenChange={() => { }}>
            <DialogContent
                className="sm:max-w-md shadow-2xl"
                onInteractOutside={(e) => e.preventDefault()}
                onEscapeKeyDown={(e) => e.preventDefault()}
            >
                <DialogHeader className="text-foreground">
                    <DialogTitle className="flex items-center gap-2 text-xl">
                        <Lock className="h-5 w-5" />
                        <span>Authentication Required</span>
                    </DialogTitle>
                    <DialogDescription className="text-muted-foreground">
                        Please sign in to access Pika.
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
                            className="bg-background border-input"
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
                            className="bg-background border-input"
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
        </Dialog>
    );
}
