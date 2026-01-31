# Frontend Styling Refactor Plan: Theme-Aware Configuration

## TL;DR

> **Quick Summary**: Refactor the frontend to eliminate hardcoded colors and light-mode-only classes, replacing them with a semantic CSS variable system that supports both "Warm Paper" (Light) and "Blackboard" (Dark) themes.
> 
> **Deliverables**:
> - Updated `index.css` with semantic variables (success, warning, thinking, etc.)
> - Updated `tailwind.config.js` mapping these variables
> - Refactored components (`Input`, `Select`, `Badge`, `SessionList`, etc.) using new tokens
> - Fully functional Dark Mode without hardcoded hex fallbacks
> 
> **Estimated Effort**: Medium (15-20 files touched)
> **Parallel Execution**: YES - 3 Waves
> **Critical Path**: Core Tokens → Tailwind Config → Component Refactoring

---

## Context

### Original Request
"Review the frontend implementation, make the styles configurable, compatible with the light/dark mode switch, and simple, robust."

### Current State Analysis
- **Framework**: React + Tailwind v4 + Shadcn UI + `next-themes`
- **Themes**: "Warm Paper" (Light) / "Blackboard" (Dark)
- **Issues Identified**:
  - Hardcoded hex: `#2d5da1` (focus), `#fff9c4` (thinking)
  - Light-only classes: `bg-green-50`, `text-green-700`
  - Forced themes: `Sonner` hardcoded to `theme="light"`
  - Overlay shadows: Hardcoded `rgba` values

### Metis/Self-Review Gaps Addressed
- **Accessibility**: Semantic tokens must ensure contrast ratios in both modes.
- **Aesthetics**: "Handwritten" feel must be preserved; avoid "corporate" flat colors.
- **Hydration**: Ensure `next-themes` handles the switch without flicker.

---

## Work Objectives

### Core Objective
Establish a robust, theme-aware styling system where *every* color is a semantic variable, enabling perfect Light/Dark mode switching without code changes.

### Concrete Deliverables
- `frontend-web/src/index.css`: Added `--success`, `--warning`, `--thinking`, `--overlay`, `--ring-focus`
- `frontend-web/tailwind.config.js`: Extended theme configuration
- Refactored Components: `Input`, `Select`, `Textarea`, `Badge`, `ThinkingIndicator`, `SessionList`, `SessionHistory`, `DiffViewer`, `AppHeader`, `Sonner`

### Definition of Done
- [ ] `grep -r "#" src/components` returns 0 results for color properties (excluding SVG fills if any)
- [ ] Dark mode toggle switches ALL UI elements instantly
- [ ] No regression in "Warm Paper" aesthetic

### Must Have
- Semantic naming (`--thinking-bg` not `--yellow-light`)
- Dark mode overrides for ALL new variables

### Must NOT Have (Guardrails)
- New hardcoded hex values
- "Magic numbers" for opacity in components (move to variables)

---

## Verification Strategy

### Automated Verification Only (NO User Intervention)

**For Token Existence (Config/Infra):**
```bash
# Verify variable existence in CSS
grep "\-\-thinking" frontend-web/src/index.css
# Verify Tailwind config map
grep "thinking" frontend-web/tailwind.config.js
```

**For Component Usage (Refactor):**
```bash
# Verify no hardcoded hex in Input
! grep "#2d5da1" frontend-web/src/components/ui/input.tsx
# Verify semantic class usage
grep "ring-focus" frontend-web/src/components/ui/input.tsx
```

**For Visual Regression (UI):**
```javascript
// Playwright: Check element styles computed values
// (Since we don't have visual regression infra, we rely on computed style checks via DOM)
const element = document.querySelector('.badge');
const style = getComputedStyle(element);
// Assert background color is not the hardcoded hex
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundational - Start Immediately):
├── Task 1: Core Tokens (index.css)
└── Task 2: Sonner Fix (Quick Fix)

Wave 2 (Configuration - After Wave 1):
└── Task 3: Tailwind Config (Depends on Task 1)

Wave 3 (Component Refactor - Parallel - After Wave 2):
├── Task 4: Form Elements (Input, Select, Textarea)
├── Task 5: Indicators (Badge, ThinkingIndicator)
├── Task 6: Overlays (Sheet, Dialog, Card)
├── Task 7: Session Views (List, History)
├── Task 8: Diff Viewer
└── Task 9: App Header
```

### Dependency Matrix

| Task | Depends On | Blocks |
|------|------------|--------|
| 1 | None | 3 |
| 2 | None | None |
| 3 | 1 | 4, 5, 6, 7, 8, 9 |
| 4-9 | 3 | None |

---

## TODOs

### Wave 1: Foundation

- [ ] 1. Define Core Semantic Tokens
  **What to do**:
  - Edit `frontend-web/src/index.css`
  - Add semantic variables to `:root` and `.dark` override
  - **New Variables Needed**:
    - `--success`, `--success-foreground` (was green-500/50/700)
    - `--warning`, `--warning-foreground` (was amber-50)
    - `--error`, `--error-foreground` (was red-50/500/700)
    - `--info`, `--info-foreground` (was blue-50/500)
    - `--thinking`, `--thinking-foreground` (was `#fff9c4`)
    - `--ring-focus` (was `#2d5da1` / `#4dabf7`)
    - `--overlay` (was `black/80`)
    - `--shadow-hard` (was `rgba(45,45,45,0.1)`)
    - `--diff-added-bg`, `--diff-added-text`
    - `--diff-removed-bg`, `--diff-removed-text`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  
  **References**:
  - `frontend-web/src/index.css` (existing vars)
  - `frontend-web/src/components/ui/input.tsx` (source of blue ring color)
  - `frontend-web/src/components/ThinkingIndicator.tsx` (source of yellow color)

  **Acceptance Criteria**:
  ```bash
  grep "\-\-thinking" frontend-web/src/index.css
  grep "\.dark" frontend-web/src/index.css -A 20 | grep "\-\-thinking"
  ```

- [ ] 2. Fix Sonner Forced Theme
  **What to do**:
  - Edit `frontend-web/src/components/ui/sonner.tsx`
  - Remove `theme="light"` prop from `Toaster` component
  - Allow it to inherit system preference or `next-themes` context

  **Recommended Agent**: `quick` (typescript-programmer)

  **Acceptance Criteria**:
  ```bash
  ! grep 'theme="light"' frontend-web/src/components/ui/sonner.tsx
  ```

### Wave 2: Configuration

- [ ] 3. Update Tailwind Configuration
  **What to do**:
  - Edit `frontend-web/tailwind.config.js`
  - Extend `theme.extend.colors`:
    - `success: "var(--success)"` (and foreground)
    - `warning: "var(--warning)"` ...
    - `thinking: "var(--thinking)"` ...
    - `diff-added: "var(--diff-added-bg)"` ...
  - Extend `theme.extend.borderColor`:
    - `focus: "var(--ring-focus)"`
  - Extend `theme.extend.ringColor`:
    - `focus: "var(--ring-focus)"`

  **Recommended Agent**: `quick` (typescript-programmer)
  **Depends On**: Task 1

  **Acceptance Criteria**:
  ```bash
  grep "thinking" frontend-web/tailwind.config.js
  grep "ring-focus" frontend-web/tailwind.config.js
  ```

### Wave 3: Component Refactoring

- [ ] 4. Refactor Form Elements (Input, Select, Textarea)
  **What to do**:
  - Replace hardcoded `#2d5da1` with `border-focus` / `ring-focus` utilities
  - Ensure `bg-background` and `text-foreground` are used correctly
  - Files:
    - `frontend-web/src/components/ui/input.tsx`
    - `frontend-web/src/components/ui/select.tsx`
    - `frontend-web/src/components/ui/textarea.tsx`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  **Depends On**: Task 3

  **Acceptance Criteria**:
  ```bash
  ! grep "#2d5da1" frontend-web/src/components/ui/input.tsx
  ! grep "#2d5da1" frontend-web/src/components/ui/select.tsx
  ! grep "#2d5da1" frontend-web/src/components/ui/textarea.tsx
  ```

- [ ] 5. Refactor Indicators (Badge, ThinkingIndicator)
  **What to do**:
  - Replace `#fff9c4` with `bg-thinking`
  - Update text colors to `text-thinking-foreground`
  - Files:
    - `frontend-web/src/components/ui/badge.tsx`
    - `frontend-web/src/components/ThinkingIndicator.tsx`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  **Depends On**: Task 3

  **Acceptance Criteria**:
  ```bash
  ! grep "#fff9c4" frontend-web/src/components/ui/badge.tsx
  grep "bg-thinking" frontend-web/src/components/ThinkingIndicator.tsx
  ```

- [ ] 6. Refactor Overlays and Shadows
  **What to do**:
  - Replace `bg-black/80` with `bg-overlay`
  - Replace `shadow-[...]` with `shadow-hard` (ensure `shadow-hard` in config uses the variable)
  - Files:
    - `frontend-web/src/components/ui/sheet.tsx`
    - `frontend-web/src/components/ui/dialog.tsx`
    - `frontend-web/src/components/ui/card.tsx`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  **Depends On**: Task 3

  **Acceptance Criteria**:
  ```bash
  ! grep "bg-black/80" frontend-web/src/components/ui/dialog.tsx
  ```

- [ ] 7. Refactor Session Views (List & History)
  **What to do**:
  - Replace `bg-green-500` (active state) with `bg-primary` or `bg-active` (check semantic fit)
  - Replace `bg-green-50`, `bg-blue-50` etc. with `bg-success/20`, `bg-info/20`
  - Fix gradients to use semantic stops
  - Files:
    - `frontend-web/src/components/SessionList.tsx`
    - `frontend-web/src/components/SessionHistory.tsx`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  **Depends On**: Task 3

  **Acceptance Criteria**:
  ```bash
  ! grep "bg-green-500" frontend-web/src/components/SessionList.tsx
  ! grep "from-blue-500" frontend-web/src/components/SessionHistory.tsx
  ```

- [ ] 8. Refactor Diff Viewer
  **What to do**:
  - Replace `bg-green-50` / `text-green-700` with `bg-diff-added` / `text-diff-added-foreground`
  - Replace `bg-red-50` / `text-red-700` with `bg-diff-removed` / `text-diff-removed-foreground`
  - Handle dark mode overrides via the variables, not conditional classes if possible (or simplified conditional classes)
  - File: `frontend-web/src/components/DiffViewer.tsx`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  **Depends On**: Task 3

  **Acceptance Criteria**:
  ```bash
  ! grep "bg-green-50" frontend-web/src/components/DiffViewer.tsx
  ```

- [ ] 9. Refactor App Header
  **What to do**:
  - Update connection status badges to use `bg-success`, `bg-warning`, `bg-error`
  - File: `frontend-web/src/components/AppHeader.tsx`

  **Recommended Agent**: `visual-engineering` (frontend-ui-ux)
  **Depends On**: Task 3

  **Acceptance Criteria**:
  ```bash
  ! grep "bg-green-50" frontend-web/src/components/AppHeader.tsx
  ```

---

## Commit Strategy

- **Commit 1**: `style(tokens): add semantic css variables and tailwind config` (Tasks 1, 2, 3)
- **Commit 2**: `refactor(ui): update core ui components to use semantic tokens` (Tasks 4, 5, 6)
- **Commit 3**: `refactor(views): update views and diffs for theme compatibility` (Tasks 7, 8, 9)

## Success Checklist
- [ ] No hardcoded hex values in component files
- [ ] No hardcoded `bg-*-50` classes in component files
- [ ] Dark mode switching works for all interactive elements
- [ ] Focus rings are visible and consistent
