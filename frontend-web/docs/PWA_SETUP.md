# PWA Setup Instructions

## Service Worker Registration

Add this to `src/main.tsx`:

```typescript
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js')
      .then(registration => {
        console.log('SW registered: ', registration);
      })
      .catch(registrationError => {
        console.log('SW registration failed: ', registrationError);
      });
  });
}
```

## Manifest Setup

Add to `index.html` in `<head>`:

```html
<link rel="manifest" href="/manifest.json" />
<meta name="theme-color" content="#000000" />
```

## Required Icons

Create icons in `public/`:
- `icon-192.png` (192x192)
- `icon-512.png` (512x512)

## Testing

1. Start dev server: `npm run dev`
2. Open DevTools > Application
3. Check "Service Workers" and "Manifest"
4. Test offline mode in Network tab

## Deployment

The service worker will be cached and served from the production build.
