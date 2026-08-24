# OffShoot Interaction Matrix

Status values: `Not captured`, `Captured`, `Implemented`, `Verified`.

| ID | Surface / behavior | Evidence required | Status | Notes |
|---|---|---|---|---|
| SHELL-01 | Main shell, three columns | Screenshot + bounds | Captured | OffShoot: Sources left, flexible Disks center, Destinations right |
| SHELL-02 | Collapse/expand Disks | Before/during/after | Not captured | |
| DISK-01 | Available disk card | Default + hover + focus | Captured | Grid in independently scrolling center panel |
| DISK-02 | Assigned disk disappears from grid | Before/after | Captured | DATA (E:) appears in Destinations, not center grid |
| DISK-03 | Removable/busy states | Screenshot + action result | Not captured | |
| MENU-01 | Available disk context menu | All items + order | Not captured | |
| MENU-02 | Source context menu | All items + order | Not captured | |
| MENU-03 | Destination context menu | All items + order | Not captured | |
| MENU-04 | Submenu hover/placement | Timed pointer path + screenshot | Not captured | |
| MENU-05 | Keyboard/focus/close | Key sequence + resulting focus | Not captured | |
| DRAG-01 | Disk → Source | Before/during/after | Not captured | |
| DRAG-02 | Disk → Destination | Before/during/after | Not captured | |
| DRAG-03 | Destination reorder/Cascade | Preview + insertion point + result | Not captured | |
| DRAG-04 | Invalid drop / cancel | Feedback + unchanged state | Not captured | |
| EDIT-01 | Inline label | Enter/Escape/blur | Not captured | |
| ACTION-01 | Add folder via `+` | Picker cancel/success | Captured partially | Empty Sources shows centered `+` |
| ACTION-02 | Rename volume | Success/error | Not captured | |
| ACTION-03 | Eject | Idle/busy/error | Not captured | |
| ACTION-04 | Hide disk | Before/after/persistence | Not captured | |
| VERIFY-01 | Verify disk/folder | Menu → progress → result | Not captured | |
| TRANSFER-01 | Parallel setup | Controls + result mapping | Not captured | |
| TRANSFER-02 | Cascade setup | Controls + hop order | Not captured | |
| TRANSFER-03 | Queue/progress/verify | All job states | Not captured | |
| TRANSFER-04 | Stop/resume/cancel | Dialog + final state | Not captured | |
| TRANSFER-05 | Failure/broken/missing files | Error and recovery | Not captured | |
| PREF-01 | Preferences | Tabs/fields/keyboard | Not captured | |
| LOG-01 | Transfer logs/reports | Open/navigation/empty/data | Not captured | |

## Confirmed shell observation

- Reference screenshot: `screenshots/offshoot-shell-main.png`.
- OffShoot window showed a dark desktop shell with:
  - header/menu/license notice;
  - Sources fixed left with dashed empty drop zone and centered `+`;
  - Disks flexible center with 3-column card grid at the observed window size;
  - Destinations fixed right;
  - independent center vertical scrolling;
  - an assigned destination (`DATA (E:)`) outside the available disk grid.
- Branding, license notice, glyphs and exact colors are not parity targets.
