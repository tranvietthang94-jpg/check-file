/** Shared MIME type for the HTML5 drag-and-drop payload used to drag a disk
 * from the Disks grid onto a Sources/Destinations drop zone. */
export const DISK_DRAG_MIME = "application/x-offloadkit-disk-id";

/** Reordering an existing row within an EndpointList. */
export const ENDPOINT_REORDER_MIME = "application/x-offloadkit-endpoint-reorder";

/** Dragging an assigned endpoint back to the disk grid removes its assignment. */
export const ENDPOINT_REMOVE_MIME = "application/x-offloadkit-endpoint-remove";
