import type { ChecksumAlgorithm, VerificationMode } from "./job";
import type { OrganizeSettings } from "./organize";

export interface Preset {
  name: string;
  verificationMode: VerificationMode;
  checksumAlgorithm: ChecksumAlgorithm;
  organize: OrganizeSettings;
}
