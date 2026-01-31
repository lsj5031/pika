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
import { storeCredentials } from "../lib/auth";
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
            // Test credentials by making a request to the API
            const encoded = btoa(`${username}:${password}`);
            const response = await fetch(`${config.API_URL}/api/projects`, {
                headers: {
                    Authorization: `Basic ${encoded}`,
                },
            });

            if (response.ok) {
                // Credentials valid - store them
                storeCredentials({ username, password });
                onAuthenticated();
            } else if (response.status === 401) {
                setError("Invalid username or password");
            } else {
                setError(`Authentication failed: ${response.statusText}`);
            }
        } catch {
            // Network error - might mean server doesn't have auth enabled
            // Store credentials anyway and let the user proceed
            storeCredentials({ username, password });
            onAuthenticated();
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
