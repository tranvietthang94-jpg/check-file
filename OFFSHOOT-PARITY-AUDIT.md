# OffloadKit — đối chiếu hành vi với OffShoot 26.2.1

Ngày audit: 2026-08-24

> Phạm vi clean-room cho nhu cầu cá nhân: đối chiếu hành vi và độ an toàn dữ liệu. Không sao chép code/branding, không phân tích hoặc triển khai license/DRM, telemetry, tài khoản cloud của OffShoot.

## Kết luận

OffloadKit đã có phần lớn **pipeline ingest cốt lõi** và không phải prototype sơ sài. Baseline trước sửa:

- `cargo test`: 151 pass, 0 fail.
- `npm run build`: pass.
- Có transfer nhiều destination theo Parallel/Cascade, 3 verification modes, MHL, logs/reports, queue, resume, disk operations, organize/presets và media metadata/thumbnail.

Tuy nhiên, chưa thể gọi là “giống 100%”. Parity hữu ích cá nhân hiện ở mức **cao cho local offload**, nhưng còn thiếu các nhóm: automatic repair, ASC MHL chain-of-custody, XXH3/XXH128, command automation, S3/Connect và một số durability hardening.

## Ma trận parity

| Nhóm | Trạng thái | Bằng chứng OffloadKit | Ghi chú |
|---|---|---|---|
| Copy source → destination | Implemented | `src-tauri/src/copy_engine.rs` | Streaming 1 MiB, staging `.ofkit-partial`, retry 3 lần |
| Multi-destination | Implemented | `src-tauri/src/cascade.rs` | Parallel và Cascade |
| Queue modes | Implemented | `src-tauri/src/queue.rs` | Off, SingleSource, SingleDestination, SingleTransfer |
| Verification | Implemented | `VerificationMode` trong `copy_engine.rs` | Transfer, Source, SourceAndDestination |
| Hash | Partial | `checksum.rs` | XXH64, MD5, SHA1, C4; thiếu XXH3/XXH128 |
| Legacy MHL | Implemented | `mhl.rs` | MHL 1.1, combined/per-file, verify lại |
| ASC MHL | Missing | Không có namespace/format ASC MHL | Chỉ cần khi chain-of-custody/interop thực sự cần |
| MHL awareness | Implemented | `load_source_mhl_index`, `reusable_checksum` | Reuse khi size+mtime+algorithm khớp |
| Automatic repair | Missing | Không có `RepairJob`/repair source selection | Khoảng trống quan trọng nhất sau safety |
| Transfer logs/reports | Implemented | `transfer_log.rs`, `reports.rs` | JSON logs, HTML report, thumbnail tùy chọn |
| Media metadata/thumbnail | Implemented/Partial | `metadata.rs`, `media_scan.rs` | ffprobe/ffmpeg + EXIF; không có mọi camera mapping riêng như OffShoot |
| Disk enumerate/eject/rename | Implemented | `disks.rs`, `eject.rs`, `volume_rename.rs` | Windows-focused |
| Presets/organize/selective copy | Implemented | `presets.rs`, `organize.rs` | Token templates, filters, junk exclusion |
| Stop/Resume | Implemented | Frontend tạo transfer mới dựa trên log/job cũ | Dùng duplicate detection và MHL awareness |
| Missing/broken media detection | Implemented | `copy_engine.rs` | Zero-byte alert + final presence sweep |
| Command automation | Missing | Không có command trigger/post-action engine | YAGNI cho dùng cá nhân trừ khi có workflow cụ thể |
| S3/Connect | Missing | Không có AWS/Connect dependency | Không cần cho máy cá nhân local-only |
| License/update/telemetry | Out of scope | — | Chủ động không làm parity |

## Lỗi an toàn đã sửa

### Root cause

Khi bật **Move**, duplicate detection trước đây coi file destination cùng `name + size + mtime` là giống nhau, sau đó xóa source ngay. Hai file khác nội dung nhưng trùng size và mtime có thể làm mất bản source.

Đường lỗi cũ:

```text
same name + same size + same mtime
→ DuplicateAction::Skip
→ Move deletes source
→ existing destination bytes were never proven identical
```

### Sửa

Ở nhánh `DuplicateAction::Skip` khi Move được bật:

1. Hash source và destination bằng algorithm đang chọn.
2. Chỉ xóa source nếu hai hash giống nhau.
3. Nếu hash khác, giữ destination cũ, copy source sang tên kế tiếp (`clip 2.mov`), verify, rồi mới xóa source.
4. Nếu không đọc/hash được một bên, báo failure và không xóa source.

Files sửa:

- `src-tauri/src/copy_engine.rs`
- `src-tauri/src/dedup.rs`

Regression test:

- `matching_size_and_mtime_but_different_bytes_are_not_skipped_or_deleted_on_move`
- RED trước sửa: source bị xóa do Skip sai.
- GREEN sau sửa: destination cũ giữ nguyên, source được copy/verify dưới tên mới rồi Move an toàn.

Fresh verification sau sửa:

```text
cargo test
152 passed; 0 failed
```

## Issue register

| ID | Vấn đề | Severity | Trạng thái | Mitigation/gate |
|---|---|---:|---|---|
| SAFE-001 | Move xóa source dựa trên size+mtime mà chưa chứng minh bytes | Critical | Resolved | Hash source+destination trước delete + regression test |
| SAFE-002 | Chưa thấy `sync_all`/durability barrier trước rename final | Major | Open | Thêm test fault/power-loss và flush/sync file + parent dir phù hợp nền tảng |
| SAFE-003 | Không có hardening rõ cho symlink/reparse/junction trong source tree | Major | Open | Test junction/symlink thoát root; reject hoặc không-follow theo policy |
| PARITY-001 | Không có automatic repair từ source/bản destination tốt | Major | Open | Thiết kế repair planner + explicit user approval trước ghi đè |
| PARITY-002 | Không ASC MHL, XXH3, XXH128 | Minor/Major tùy interop | Open | Chỉ thêm khi có file mẫu/interop requirement |
| PARITY-003 | Không command automation | Minor | Accepted for personal scope | Thêm khi có post-transfer workflow cụ thể |
| PARITY-004 | Không S3/Connect | Minor | Accepted for local-only scope | Không thêm nếu chỉ dùng máy cá nhân |
| DOC-001 | README vẫn là nội dung template Tauri | Minor | Open | Viết hướng dẫn thực tế trước release tiếp theo |

## Roadmap khuyến nghị

### P0 — Safety trước parity

- [x] Sửa Move duplicate false-positive.
- [ ] Atomic/durable finalization: flush + sync staging trước final rename.
- [ ] Junction/reparse/symlink escape tests trên Windows.
- [ ] Test rút destination giữa copy, verify và final rename.
- [ ] Test source thay đổi giữa scan và copy; ghi rõ kết quả là modified/failed.

### P1 — Repair workflow

- [ ] Từ MHL/log, xác định file Missing/Mismatch/SizeMismatch.
- [ ] Tìm nguồn tốt theo thứ tự: source gốc → destination sibling đã verify.
- [ ] Hash nguồn repair trước khi dùng.
- [ ] Copy vào staging, verify destination, atomic replace chỉ khi user chấp thuận.
- [ ] Không bao giờ repair đè file mà không giữ evidence/log.

### P2 — Interoperability

- [ ] Thêm XXH3/XXH128 nếu có nhu cầu trao đổi MHL với công cụ khác.
- [ ] ASC MHL chỉ triển khai sau khi có mẫu thật và validator độc lập.
- [ ] Không đoán schema/tag từ code decompile.

### Skip cho phạm vi cá nhân hiện tại

- License/DRM parity.
- Branding/UI clone 1:1.
- Telemetry/Sentry/PostHog.
- Sync Factory Connect.
- S3, trừ khi Rin thật sự offload trực tiếp lên cloud.

## Review status

- **Deliverable review:** approved with conditions.
- **Local copy core:** usable sau full test, nhưng production-grade data safety vẫn cần SAFE-002 và SAFE-003.
- **“100% OffShoot parity”:** blocked vì chưa có automatic repair, ASC MHL, XXH3/128 và một số feature thương mại/cloud; đồng thời một số parity không có giá trị cho nhu cầu cá nhân.
