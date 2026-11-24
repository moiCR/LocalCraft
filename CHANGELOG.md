All notable changes to LocalCraft are documented in this file.

## v1.1.0 — UI Redesign & Routing Overhaul

### UI & Navigation
- Replaced the collapsible sidebar with an adaptive GTK-style headerbar featuring integrated navigation.
- Added a persistent spatial server canvas (`topology.json`) with draggable server nodes, zoom/pan controls, and visual server links.
- Switched dark and light themes to clean neutral monochrome palettes aligned with the LocalCraft logo.
- Replaced verbose action buttons with circular icon-only buttons and tooltips across servers, Playit, runtimes, and console controls.
- Redesigned modals, cards, inputs, progress indicators, and empty states with consistent rounded styling and animations.

### Routing & Architecture
- Migrated routing from `gpui-router` to `gpui-pages` with directory-based route discovery.
- Moved server views to dynamic `/servers/:id` routes with a persistent nested layout shared across Console, Logs, and Files.
- Consolidated `views/` and `widgets/` into `pages/` and `components/`.

### Fixes & Improvements
- Fixed server subpage height collapse when switching between Console, Logs, and Files during route transitions.
- Made the file editor modal and file dialogs responsive to small window sizes and prevented header clipping via deferred overlays.
- Updated release packaging inputs and RPM license metadata.
