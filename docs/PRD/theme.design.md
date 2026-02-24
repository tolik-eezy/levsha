# Levsha OS — Theme Design System

**Warm. Handcrafted. Copper and parchment.**

Version 1.2 · Phase 1 MVP

---

## 1. Design Philosophy

Levsha OS should feel like a craftsman's workbench in warm lamplight — copper tools, worn leather, the glow of careful metalwork. The palette draws from the world of its namesake: aged parchment, hammered copper, polished brass, and the quiet green of a workshop garden seen through the window.

The interface is never cold, never clinical, never dark. It breathes.

Principles:
- **Warm light** — cream and parchment tones, lighter and calmer than you think necessary
- **Copper craft** — the primary accent is warm copper, the color of the craftsman's tools
- **Earthy and organic** — browns, ambers, warm greens — nothing synthetic or digital
- **Watercolor calm** — colors are washed, muted, never saturated or loud
- **Generous space** — let the parchment breathe; emptiness is warmth

### 1.1 Visual Reference

Inspired by the Levsha character art and the brand's CSS identity. The light theme is the warm inverse of the dark brand palette: where the brand uses deep earth (#1C1812) as background, the OS uses sunlit parchment (#FBF8F3). The copper (#C67A52), gold (#D4A853), and green (#62B37B) accents carry through unchanged — they are the soul of the brand.

### 1.2 Dark Brand Palette (Reference Only)

These are the brand's canonical dark-mode values. The OS theme is their **light inverse** — same accent colors, inverted surfaces.

```css
--bg-dark: #1C1812;
--bg-darker: #15120E;
--bg-card: #2A2318;
--border-card: #3A3228;
--border-subtle: #2A231880;
--text-primary: #FAF8F4;
--text-secondary: #A89B8C;
--text-muted: #6B5D4F;
--accent-copper: #C67A52;
--accent-gold: #D4A853;
--accent-green: #62B37B;
```

---

## 2. Color Palette

### 2.1 Base Colors — Light Parchment

| Token | Hex | Usage |
|-------|-----|-------|
| `bg-primary` | `#FBF8F3` | Main background — warm parchment in soft light |
| `bg-secondary` | `#F5F1EA` | Input field, status bar — slightly deeper parchment |
| `bg-tertiary` | `#EBE6DC` | Borders, dividers, hover states — gentle tan |
| `bg-code` | `#F7F4EE` | Code block background — warm off-white |
| `bg-surface` | `#FDFBF7` | Assistant message bubbles — lightest parchment |

### 2.2 Text Colors — Warm Browns

| Token | Hex | Usage |
|-------|-----|-------|
| `text-primary` | `#3A3228` | Main body text — rich warm brown (from brand border-card) |
| `text-secondary` | `#6B5D4F` | Timestamps, labels — warm mid-brown (from brand text-muted) |
| `text-tertiary` | `#A89B8C` | Placeholder text, disabled — warm tan (from brand text-secondary) |
| `text-code` | `#4A3F35` | Code text — dark earth brown (from brand text-faint, darkened) |
| `text-inverse` | `#FAF8F4` | Text on colored backgrounds (from brand text-primary) |

### 2.3 Accent Colors — Copper

The primary accent is copper — the color of the craftsman's hammer, warm and alive. It is the Levsha brand color.

| Token | Hex | Usage |
|-------|-----|-------|
| `accent-copper` | `#C67A52` | Links, active elements, primary buttons — warm copper |
| `accent-copper-hover` | `#B56B45` | Hover state — deeper copper |
| `accent-copper-subtle` | `#FBF2ED` | Copper tint background — barely-there warm wash |
| `accent-copper-alpha` | `#C67A5225` | Copper at 15% opacity — for subtle highlights |

### 2.4 Secondary Accent — Gold

| Token | Hex | Usage |
|-------|-----|-------|
| `accent-gold` | `#D4A853` | Secondary accent, highlights — polished brass |
| `accent-gold-hover` | `#C49A47` | Hover state — deeper gold |
| `accent-gold-subtle` | `#FBF6EC` | Gold tint background — whisper of warmth |

### 2.5 Tertiary Accent — Green

| Token | Hex | Usage |
|-------|-----|-------|
| `accent-green` | `#62B37B` | Success states, positive indicators — workshop garden green |
| `accent-green-hover` | `#55A06D` | Hover state — deeper green |
| `accent-green-subtle` | `#EFF7F2` | Green tint background — faintest green wash |

### 2.6 Semantic Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `success` | `#62B37B` | Success — green (same as accent-green) |
| `success-bg` | `#EFF7F2` | Success background — faint green wash |
| `warning` | `#D4A853` | Warnings — gold (same as accent-gold) |
| `warning-bg` | `#FBF6EC` | Warning background — pale gold wash |
| `error` | `#C67A52` | Errors — copper (same as accent-copper) |
| `error-bg` | `#FBF2ED` | Error background — faint copper wash |
| `info` | `#8A7F72` | Info — warm brown, understated |
| `info-bg` | `#F5F1EA` | Info background — secondary parchment |

### 2.7 Destructive Command Confirmation

Uses copper, but with stronger emphasis. Not red — warm, but firm.

| Token | Hex | Usage |
|-------|-----|-------|
| `danger-border` | `#C67A52` | Destructive prompt border — copper |
| `danger-bg` | `#FDF5F0` | Destructive prompt background — warm blush parchment |
| `danger-text` | `#7A4A35` | Destructive prompt text — dark copper brown |

### 2.8 Syntax Highlighting (Code Blocks)

Muted, warm tones. The three brand accents (copper, gold, green) appear as highlights against parchment.

| Token | Hex | Element |
|-------|-----|---------|
| `syn-keyword` | `#C67A52` | Keywords — copper |
| `syn-string` | `#62B37B` | Strings — green |
| `syn-number` | `#D4A853` | Numbers — gold |
| `syn-comment` | `#A89B8C` | Comments — text-tertiary |
| `syn-function` | `#B56B45` | Functions — deeper copper |
| `syn-type` | `#55A06D` | Types — deeper green |
| `syn-operator` | `#6B5D4F` | Operators — text-secondary |
| `syn-variable` | `#4A3F35` | Variables — text-code |

### 2.9 Brand Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `brand-copper` | `#C67A52` | The Levsha diamond mark |
| `brand-gold` | `#D4A853` | Secondary brand accent |
| `brand-green` | `#62B37B` | Tertiary brand accent |

---

## 3. Typography

### 3.1 Font Stack

| Context | Font | Fallback |
|---------|------|----------|
| **UI / Conversation** | IBM Plex Sans | system-ui, -apple-system, sans-serif |
| **Code** | IBM Plex Mono | JetBrains Mono, ui-monospace, monospace |

IBM Plex Sans is the brand typeface — humanist, warm, highly legible. Its open letterforms and generous x-height give it a friendly, handcrafted quality that fits the Levsha identity. IBM Plex Mono completes the family for code.

### 3.2 Type Scale

| Token | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| `text-xs` | 11px | 400 | 1.5 | Timestamps, metadata |
| `text-sm` | 13px | 400 | 1.5 | Status bar, labels |
| `text-base` | 15px | 400 | 1.7 | Conversation text |
| `text-code` | 14px | 400 | 1.6 | Inline and block code |
| `text-lg` | 18px | 500 | 1.6 | Section headers in responses |
| `text-xl` | 24px | 500 | 1.4 | Welcome message title |
| `text-label` | 12px | 500 | 1.4 | Uppercase labels, badges |

### 3.3 Font Rendering

- Subpixel antialiasing: **enabled**
- Hinting: **slight** (preserves the soft, organic feel)
- Letter spacing: `0` for body, `0.02em` for labels
- Paragraph spacing: `1.2em` between message paragraphs
- Text color is warm brown (`#3A3228`), never pure black

---

## 4. Spacing System

Base unit: **4px**

| Token | Value | Usage |
|-------|-------|-------|
| `space-xs` | 4px | Tight internal padding |
| `space-sm` | 8px | Between inline elements |
| `space-md` | 16px | Standard padding, message gaps |
| `space-lg` | 24px | Section spacing |
| `space-xl` | 32px | Screen-edge padding |
| `space-2xl` | 48px | Major section breaks |

---

## 5. Component Styles

### 5.1 Message Bubbles

```
Assistant message:
  background: bg-surface (#FDFBF7)
  border: 1px solid bg-tertiary (#EBE6DC)
  border-radius: 8px
  padding: space-md (16px)
  shadow: 0 1px 3px rgba(58, 50, 40, 0.04)  -- warm-tinted shadow

User message:
  background: bg-secondary (#F5F1EA)
  border: none
  border-radius: 8px
  padding: space-md (16px)

Message gap: space-md (16px)
Max width: 720px, centered
```

### 5.2 Input Field

```
background: bg-secondary (#F5F1EA)
border: 1px solid bg-tertiary (#EBE6DC)
border-focus: 1px solid accent-copper (#C67A52)
border-radius: 12px
padding: 12px 16px
font: text-base (15px IBM Plex Sans)
color: text-primary (#3A3228)
placeholder-color: text-tertiary (#A89B8C)
min-height: 44px
max-height: 200px (multi-line expansion)
margin: 0 space-xl (32px) space-lg (24px)
```

### 5.3 Status Bar

```
background: bg-secondary (#F5F1EA)
border-top: 1px solid bg-tertiary (#EBE6DC)
height: 32px
padding: 0 space-md (16px)
font: text-sm (13px IBM Plex Sans)
color: text-secondary (#6B5D4F)

Items: model indicator | connection status | time
Separator: · (middle dot, text-tertiary)
```

### 5.4 Code Blocks

```
background: bg-code (#F7F4EE)
border: 1px solid bg-tertiary (#EBE6DC)
border-radius: 6px
padding: space-md (16px)
font: text-code (14px IBM Plex Mono)
color: text-code (#4A3F35)
overflow-x: auto
```

### 5.5 Tables

```
header-background: bg-secondary (#F5F1EA)
header-font-weight: 500
header-color: text-primary (#3A3228)
cell-padding: 8px 12px
cell-color: text-primary (#3A3228)
border: 1px solid bg-tertiary (#EBE6DC)
border-radius: 6px (outer)
font: text-sm (13px IBM Plex Sans)
```

### 5.6 Progress Indicators

```
Spinner:
  color: accent-copper (#C67A52)
  size: 16px
  stroke: 2px

Progress bar:
  track: bg-tertiary (#EBE6DC)
  fill: accent-copper (#C67A52)
  height: 4px
  border-radius: 2px

Status icons:
  pending: ◐ text-secondary (#6B5D4F)
  success: ✓ accent-green (#62B37B)
  failure: ✗ accent-copper (#C67A52)
```

### 5.7 Destructive Command Confirmation

```
background: danger-bg (#FDF5F0)
border: 2px solid danger-border (#C67A52)
border-radius: 8px
padding: space-md (16px)
text-color: danger-text (#7A4A35)
font-weight: 500

Buttons:
  Confirm: bg accent-copper (#C67A52), text text-inverse (#FAF8F4), border-radius 6px
  Cancel: bg bg-secondary (#F5F1EA), text text-primary (#3A3228), border 1px bg-tertiary
```

### 5.8 Error Display

```
background: error-bg (#FBF2ED)
border: 1px solid accent-copper (#C67A52)
border-radius: 8px
padding: space-md (16px)
text-color: text-primary (#3A3228)
icon-color: accent-copper (#C67A52)

Retry button:
  bg: accent-copper (#C67A52)
  text: text-inverse (#FAF8F4)
  border-radius: 6px
  padding: 8px 16px
```

---

## 6. Animation & Motion

All motion is subtle and purposeful. Like watching a craftsman's hands — deliberate, unhurried.

| Animation | Duration | Easing | Description |
|-----------|----------|--------|-------------|
| Message appear | 200ms | ease-out | Fade in + translate up 8px |
| Streaming token | 0ms | — | Instant append, no per-token animation |
| Scroll (smooth) | 300ms | ease-in-out | Auto-scroll to new content |
| Input focus | 150ms | ease-out | Border color transition to copper |
| Status change | 200ms | ease-out | Status bar text fade |
| Error appear | 250ms | ease-out | Slide down + fade in |
| Spinner | 800ms | linear | Continuous rotation, copper |
| Button hover | 100ms | ease-out | Background color shift |

---

## 7. Boot Splash

```
background: bg-primary (#FBF8F3)

Logo: ◆ Levsha OS
  diamond: accent-copper (#C67A52)
  text: text-primary (#3A3228), IBM Plex Sans, 28px, weight 600
  centered vertically and horizontally

Progress indicator:
  style: 3 small dots, sequential fade animation
  color: accent-copper (#C67A52)
  position: centered, 48px below logo
  dot-size: 6px
  dot-gap: 12px
  animation: 1200ms cycle, each dot fades 400ms

Overall: warm parchment background, copper accents.
Minimal, centered, calm. No loading bars, no percentages.
```

---

## 8. Welcome Screen

```
background: bg-primary (#FBF8F3)

Title: ◆ Levsha OS
  diamond: accent-copper (#C67A52)
  text: text-xl (24px), weight 500, text-primary (#3A3228)
  font: IBM Plex Sans
  margin-bottom: space-lg (24px)

Body text:
  font: text-base (15px IBM Plex Sans), text-primary (#3A3228)
  line-height: 1.7
  max-width: 480px, centered

  "Welcome. I'm your operating system.
   Everything you need — just ask.

   I can manage your packages and tell you about your system.
   More skills are coming soon."

Input field appears below, ready for first message.
Status bar at bottom.
```

---

## 9. Responsive Behavior

| Viewport | Layout Adjustment |
|----------|-------------------|
| < 1024px wide | Message max-width: 100% - 32px padding |
| >= 1024px wide | Message max-width: 720px, centered |
| >= 1920px wide | Message max-width: 800px, centered |

---

## 10. Iconography

No custom icon set in MVP. Use Unicode symbols consistently:

| Symbol | Color | Usage |
|--------|-------|-------|
| `◆` | accent-copper (#C67A52) | Levsha OS brand mark |
| `◐` | text-secondary (#6B5D4F) | In-progress / loading |
| `✓` | accent-green (#62B37B) | Success |
| `✗` | accent-copper (#C67A52) | Failure |
| `▸` | text-secondary (#6B5D4F) | Status bar item prefix |
| `→` | text-secondary (#6B5D4F) | Navigation hint |
| `⚠` | accent-gold (#D4A853) | Warning |

---

## 11. Accessibility Notes (MVP)

- Text contrast ratio: minimum 4.5:1 for body text
  - Verified: `#3A3228` on `#FBF8F3` = 11.8:1 (passes AAA)
  - Verified: `#6B5D4F` on `#FBF8F3` = 5.2:1 (passes AA)
  - Verified: `#C67A52` on `#FBF8F3` = 3.6:1 (large text / decorative only; paired with text labels)
- Interactive element minimum size: 44px touch target
- Focus states: `2px solid accent-copper (#C67A52)` outline with `2px offset`
- No information conveyed by color alone — always paired with text or icon

---

## 12. Theme Summary

The Levsha OS visual identity in one sentence: **warm parchment, copper tools, gold light — a craftsman's workshop where the screen disappears.**

| Element | Color | Feel |
|---------|-------|------|
| Background | Warm parchment `#FBF8F3` | Morning light on old paper |
| Text | Dark brown `#3A3228` | Ink, not pixels |
| Primary accent | Copper `#C67A52` | The craftsman's hammer |
| Secondary accent | Gold `#D4A853` | Polished brass fittings |
| Success | Green `#62B37B` | The garden through the window |
| Code background | Light parchment `#F7F4EE` | Workshop notes |
| Font | IBM Plex Sans | Warm, open, humanist |

---

## 13. CSS Variable Mapping

For implementation reference, the light theme maps to CSS custom properties:

```css
:root {
  /* Surfaces */
  --bg-primary: #FBF8F3;
  --bg-secondary: #F5F1EA;
  --bg-tertiary: #EBE6DC;
  --bg-code: #F7F4EE;
  --bg-surface: #FDFBF7;

  /* Text */
  --text-primary: #3A3228;
  --text-secondary: #6B5D4F;
  --text-tertiary: #A89B8C;
  --text-code: #4A3F35;
  --text-inverse: #FAF8F4;

  /* Accents */
  --accent-copper: #C67A52;
  --accent-copper-hover: #B56B45;
  --accent-copper-subtle: #FBF2ED;
  --accent-gold: #D4A853;
  --accent-gold-subtle: #FBF6EC;
  --accent-green: #62B37B;
  --accent-green-subtle: #EFF7F2;

  /* Borders */
  --border-primary: #EBE6DC;
  --border-subtle: #EBE6DC80;

  /* Font */
  font-family: "IBM Plex Sans", system-ui, sans-serif;
}
```

---

*All design.md files across modules MUST reference this theme document for colors, typography, spacing, and component styles. No module may define its own color palette or override these tokens. The theme is LIGHT — there is no dark mode in MVP.*
