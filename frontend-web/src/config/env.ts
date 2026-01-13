// Environment variables are automatically loaded by Vite from .env files
// Variables must be prefixed with VITE_ to be available in the browser

const API_URL = import.meta.env.VITE_API_URL || "http://localhost:8080/api";
const WS_URL = import.meta.env.VITE_WS_URL || "ws://localhost:8080/ws";

export const config = {
  API_URL,
  WS_URL,
} as const;

export type Config = typeof config;
