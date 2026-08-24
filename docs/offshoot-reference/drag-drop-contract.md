# OffShoot Drag/Drop Contract

> Status: interaction capture pending. The shell screenshot confirms dashed Source drop-zone and separate Destination column; exact ghost, thresholds and insertion marker remain `Unknown` until observed.

## Disk → Source

- Preconditions: disk is visible in the available Disks grid and is not assigned.
- Capture:
  - pointer-down origin;
  - drag-start threshold;
  - disk ghost/preview;
  - Source valid-target state;
  - invalid-target state;
  - final Source card;
  - disk removal from center grid;
  - focus/selection after drop.

## Disk → Destination

Capture the same lifecycle and confirm whether the destination is appended, inserted at a pointer position, or opens additional configuration.

## Destination reorder / Cascade

- Capture whether reorder is whole-card or handle-only.
- Capture before/after insertion marker.
- Confirm whether drop position is before/after target based on pointer midpoint.
- Confirm displayed order is the Cascade hop order.
- Confirm dropping onto itself/outside is a no-op.

## Cancel and long-list behavior

- Escape cancellation.
- Drop outside all targets.
- Drag near scroll-region top/bottom.
- Disk already assigned or hidden.
- Disconnected disk during drag.

## OffloadKit acceptance criteria

- Source data remains unchanged until a valid drop commits.
- Invalid/unknown MIME payload cannot mutate stores.
- Assigned disk cannot be duplicated in one endpoint list.
- A drop outside leaves state unchanged.
- Destination reorder has an explicit insertion marker.
- Drag end always clears highlight/preview/autoscroll state.
- Keyboard alternative exists for destination reorder.
