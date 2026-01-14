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
            const response = await fetch("/api/projects", {
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
        } catch (err) {
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
                className="sm:max-w-md"
                onInteractOutside={(e) => e.preventDefault()}
                onEscapeKeyDown={(e) => e.preventDefault()}
            >
                <DialogHeader>
                    <DialogTitle className="flex items-center gap-2">
                        <Lock className="h-5 w-5" />
                        Authentication Required
                    </DialogTitle>
                    <DialogDescription>
                        Please enter your credentials to access Pika.
                    </DialogDescription>
                </DialogHeader>

                <div className="grid gap-4 py-4">
                    <div className="grid gap-2">
                        <Label htmlFor="auth-username">Username</Label>
                        <Input
                            id="auth-username"
                            type="text"
                            placeholder="Enter username"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            onKeyDown={handleKeyDown}
                            autoComplete="username"
                            autoFocus
                        />
                    </div>

                    <div className="grid gap-2">
                        <Label htmlFor="auth-password">Password</Label>
                        <Input
                            id="auth-password"
                            type="password"
                            placeholder="Enter password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            onKeyDown={handleKeyDown}
                            autoComplete="current-password"
                        />
                    </div>

                    {error && (
                        <p className="text-sm text-destructive font-medium">{error}</p>
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
