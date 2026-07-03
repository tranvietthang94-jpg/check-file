export interface ReportRequest {
  jobIds: string[];
  title: string;
  notes: string;
  logoDataUrl: string | null;
  includeThumbnails: boolean;
}
