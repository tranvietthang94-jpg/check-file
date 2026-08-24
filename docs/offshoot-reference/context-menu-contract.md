# OffShoot Context Menu Contract

> Status: partial capture. Exact menu labels/order remain `Unknown` until the native menu can be captured reliably. Existing OffloadKit labels must not be treated as reference evidence.

## Surfaces

- Available disk in center grid.
- Assigned Source endpoint.
- Assigned Destination endpoint.
- Removable disk.
- Busy disk participating in a queued/running transfer.

## Behaviors to capture

| Behavior | Expected evidence | Status |
|---|---|---|
| Right-click opens at pointer | Screenshot + bounds | Pending |
| Menu stays inside viewport | Four edge cases | Pending |
| Item order and separators | Screenshot/transcription | Pending |
| Disabled/checked/danger states | Per disk state | Pending |
| Hover opens submenu | Timing + screenshot | Pending |
| Pointer can enter submenu without closing | Pointer trajectory | Pending |
| Click outside closes | Before/after | Pending |
| Escape closes | Before/after + focus | Pending |
| Right-click another disk relocates menu | Before/after | Pending |
| Arrow/Enter navigation | Key sequence + focus | Pending |

## Confirmed from OffloadKit current implementation, not OffShoot reference

The current app already supports a custom menu with nested children, click-outside closing and Escape closing. Known parity gaps to test/fix are viewport collision, keyboard navigation, focus restoration, submenu timing/corridor, separators/checked state and exact per-state content.

## Safety constraints

- Eject must be disabled while the disk is busy.
- Hide must not remove an assigned disk.
- Rename volume must remain distinct from an app-only label.
- Verify must navigate to/report actual MHL verification and must not report success optimistically.
