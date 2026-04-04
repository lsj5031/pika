import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SessionItem } from "../SessionItem";
import type { Session } from "../../types";

const mockSession: Session = {
  id: "session-123",
  name: "Test Session",
  project_id: "project-1",
  project_path: "/test/path",
  created_at: "2026-01-18T10:00:00Z",
  is_active: false,
};

describe("SessionItem", () => {
  it("renders session name", () => {
    render(
      <SessionItem
        session={mockSession}
        isSelected={false}
        isActive={false}
        isThinking={false}
        isUnread={false}
        isFavorite={false}
        onClick={vi.fn()}
      />
    );
    expect(screen.getByText("Test Session")).toBeInTheDocument();
  });

  it("renders the favorite button in the DOM when showFavoriteButton is true", () => {
    render(
      <SessionItem
        session={mockSession}
        isSelected={false}
        isActive={false}
        isThinking={false}
        isUnread={false}
        isFavorite={false}
        showFavoriteButton={true}
        onToggleFavorite={vi.fn()}
        onClick={vi.fn()}
      />
    );
    // The favorite button should be in the DOM (even if opacity-0 via CSS)
    // It uses the Star icon - look for a button with an svg inside
    const buttons = screen.getAllByRole("button");
    // The outer button is the session click handler, there should be a favorite button too
    const favoriteButton = buttons.find(
      (btn) => btn.querySelector("svg.lucide-star") !== null
    );
    expect(favoriteButton).toBeTruthy();
  });

  it("has the 'group' CSS class on the parent container for the non-compact layout so favorite button hover works", () => {
    const { container } = render(
      <SessionItem
        session={mockSession}
        isSelected={false}
        isActive={false}
        isThinking={false}
        isUnread={false}
        isFavorite={false}
        showFavoriteButton={true}
        onToggleFavorite={vi.fn()}
        onClick={vi.fn()}
      />
    );
    // The favorite button uses group-hover:opacity-100, so its parent must have 'group' class
    const favoriteButton = screen.getAllByRole("button").find(
      (btn) => btn.querySelector("svg.lucide-star") !== null
    );
    expect(favoriteButton).toBeTruthy();

    // Walk up from the favorite button to find the parent with the 'group' class
    // The favorite button's className includes 'group-hover:opacity-100'
    // Its closest ancestor div needs the 'group' class
    let parent = favoriteButton!.parentElement;
    let foundGroupClass = false;
    while (parent) {
      if (parent.classList.contains("group")) {
        foundGroupClass = true;
        break;
      }
      parent = parent.parentElement;
    }
    expect(foundGroupClass).toBe(true);
  });
});
