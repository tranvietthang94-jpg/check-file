/** Shared MIME type for the HTML5 drag-and-drop payload used to drag a disk
 * from the Disks grid onto a Sources/Destinations drop zone. */
export const DISK_DRAG_MIME = "application/x-offloadkit-disk-id";

/** Reordering an existing row within an EndpointList (currently only wired
 * up for Destinations, where the list order doubles as Cascade hop order). */
export const ENDPOINT_REORDER_MIME = "application/x-offloadkit-endpoint-reorder";
