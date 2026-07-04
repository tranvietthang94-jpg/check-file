---
name: OffloadKit
colors:
  surface: '#131313'
  surface-dim: '#131313'
  surface-bright: '#3a3939'
  surface-container-lowest: '#0e0e0e'
  surface-container-low: '#1c1b1b'
  surface-container: '#201f1f'
  surface-container-high: '#2a2a2a'
  surface-container-highest: '#353534'
  on-surface: '#e5e2e1'
  on-surface-variant: '#c4c7c8'
  inverse-surface: '#e5e2e1'
  inverse-on-surface: '#313030'
  outline: '#8e9192'
  outline-variant: '#444748'
  surface-tint: '#c6c6c7'
  primary: '#ffffff'
  on-primary: '#2f3131'
  primary-container: '#e2e2e2'
  on-primary-container: '#636565'
  inverse-primary: '#5d5f5f'
  secondary: '#84ff92'
  on-secondary: '#003910'
  secondary-container: '#03e85c'
  on-secondary-container: '#006222'
  tertiary: '#ffffff'
  on-tertiary: '#690003'
  tertiary-container: '#ffdad5'
  on-tertiary-container: '#ca0a0f'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#e2e2e2'
  primary-fixed-dim: '#c6c6c7'
  on-primary-fixed: '#1a1c1c'
  on-primary-fixed-variant: '#454747'
  secondary-fixed: '#6bff83'
  secondary-fixed-dim: '#00e55a'
  on-secondary-fixed: '#002107'
  on-secondary-fixed-variant: '#00531b'
  tertiary-fixed: '#ffdad5'
  tertiary-fixed-dim: '#ffb4aa'
  on-tertiary-fixed: '#410001'
  on-tertiary-fixed-variant: '#930005'
  background: '#131313'
  on-background: '#e5e2e1'
  surface-variant: '#353534'
typography:
  headline-lg:
    fontFamily: Hanken Grotesk
    fontSize: 32px
    fontWeight: '700'
    lineHeight: '1.1'
    letterSpacing: -0.02em
  headline-lg-mobile:
    fontFamily: Hanken Grotesk
    fontSize: 24px
    fontWeight: '700'
    lineHeight: '1.1'
  headline-md:
    fontFamily: Hanken Grotesk
    fontSize: 20px
    fontWeight: '600'
    lineHeight: '1.2'
  body-lg:
    fontFamily: Hanken Grotesk
    fontSize: 16px
    fontWeight: '400'
    lineHeight: '1.5'
  body-sm:
    fontFamily: Hanken Grotesk
    fontSize: 14px
    fontWeight: '400'
    lineHeight: '1.5'
  mono-data:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: '500'
    lineHeight: '1.4'
  label-caps:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: '700'
    lineHeight: '1'
    letterSpacing: 0.05em
spacing:
  unit: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 48px
  gutter: 12px
  margin: 16px
---

## Brand & Style

The brand personality is clinical, precise, and uncompromising. Designed for Digital Imaging Technicians (DITs) and data managers, the visual language prioritizes utility and speed over aesthetics. It adopts a **High-Contrast Brutalist** style, characterized by pure black backgrounds, needle-thin white borders, and an aggressive focus on information density.

The UI should feel like a high-end piece of hardware—reliable and immediate. There are no gradients, no soft shadows, and no decorative flourishes. Every pixel serves the purpose of monitoring data integrity and offload progress. The emotional response should be one of absolute confidence and technical control.

## Colors

This design system utilizes a high-contrast palette optimized for low-light onset environments.

- **Background:** Pure Black (#000000) is used for the base layer to eliminate light bleed and maximize contrast.
- **Surface:** Deep Charcoal (#111111) is used for secondary containers and panel backgrounds to create subtle separation.
- **Primary:** White (#FFFFFF) is used for all core UI borders, primary labels, and icons.
- **Success/Action:** Vibrant Green (#19ED60) is reserved for active transfer states, verified checksums, and "Go" actions.
- **Error/Alert:** Pure Red (#FF3B30) is used exclusively for failed transfers, corrupted files, and critical system warnings.

## Typography

The typography system is split between functional UI navigation and technical data display.

- **Hanken Grotesk** is used for the primary interface: headlines, navigation, and general body text. It provides a sharp, modern sans-serif feel that remains legible even at high densities.
- **JetBrains Mono** is used for all technical data, including file paths, checksum strings, timecodes, and metadata. The monospaced nature ensures that columns of numbers and strings align perfectly for quick scanning.

All labels should be set in uppercase with slight letter spacing to emphasize the industrial, tool-like nature of the application.

## Layout & Spacing

This design system uses a **High-Density Fluid Grid** model. Given the professional nature of data management, information density is prioritized over whitespace.

- **Grid:** A 12-column fluid grid for desktop, collapsing to a single column for mobile. 
- **Rhythm:** A strict 4px baseline grid governs all vertical spacing.
- **Gutters:** Tight 12px gutters maximize screen real estate for multi-column data views.
- **Padding:** Internal container padding is generally set to `sm` (8px) or `md` (16px) to keep elements compact.

On mobile, the layout reflows to stack panels, but text sizes and border weights remain constant to maintain the "precision tool" aesthetic.

## Elevation & Depth

Depth is conveyed through **Low-Contrast Outlines** and tonal layering rather than shadows. 

- **Level 0 (Background):** Pure Black (#000000).
- **Level 1 (Panels):** Deep Charcoal (#111111) with a 1px White (#FFFFFF) border.
- **Level 2 (Active/Hover):** A subtle shift to a slightly lighter charcoal or a solid 1px Green border for active selections.

Shadows are strictly forbidden. To indicate that a component is "above" another (like a dropdown or modal), use a solid White 1px border and a Pure Black background to "cut through" the underlying content.

## Shapes

The shape language is **strictly geometric and sharp**. 

All buttons, input fields, containers, and cards must have a 0px border radius. This reinforces the professional, industrial aesthetic and ensures that 1px borders align perfectly with the pixel grid, eliminating anti-aliasing blur on high-resolution displays.

## Components

### Buttons
Buttons are rectangular with 0px corners.
- **Primary:** Black background, 1px White border, White text.
- **Action (Success):** Green background (#19ED60), Black text.
- **Ghost:** Transparent background, 1px White border (30% opacity), White text.

### Inputs
Input fields consist of a 1px White border with a JetBrains Mono typeface. Labels sit strictly above the input in `label-caps`. Focus state is indicated by the border changing to 100% opacity or Green (#19ED60).

### Data Tables
Tables are the core of the system. Use 1px horizontal dividers. Header cells should have a Deep Charcoal background to differentiate from data rows. All numerical data must use the `mono-data` typography style.

### Progress Bars
Progress bars are flat. The track is Deep Charcoal, and the fill is Vibrant Green. No rounded ends. For errors, the fill color switches to Pure Red.

### Status Chips
Small, rectangular boxes with a 1px border. No background fill unless the status is "Active" or "Error." Use JetBrains Mono for the text inside.