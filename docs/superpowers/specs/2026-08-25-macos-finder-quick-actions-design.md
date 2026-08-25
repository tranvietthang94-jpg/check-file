# Phase 14 — macOS Finder Quick Actions Design

## Mục tiêu

Mang bốn workflow của Windows Explorer sang macOS Finder bằng Quick Actions cài theo user:

1. Đặt selection làm Source trong OffloadKit.
2. Đặt một folder làm Destination.
3. Copy selection bằng OffloadKit.
4. Paste selection vào folder đích và bắt đầu transfer an toàn.

Ứng dụng được kéo vào `/Applications/OffloadKit.app`; người dùng bật/tắt integration trong Preferences.

## Phương án

Dùng Automator Quick Action bundles (`.workflow`) được tạo bởi OffloadKit và cài vào:

```text
~/Library/Services/
```

Không dùng Finder Sync Extension vì Phase này không cần badge/sync state, còn Finder Sync yêu cầu Xcode extension target, signing và approval phức tạp hơn.

## Finder menu

### File/folder selection

```text
Quick Actions
├── Đặt làm Source trong OffloadKit
└── Copy bằng OffloadKit
```

### Một folder

```text
Quick Actions
├── Đặt làm Source trong OffloadKit
├── Đặt làm Destination trong OffloadKit
├── Copy bằng OffloadKit
└── Paste và bắt đầu transfer
```

Phase đầu không hỗ trợ contextual menu trên vùng trống Finder. Người dùng tác động trực tiếp lên folder đích.

## Activation contract

Workflow nhận POSIX paths từ Finder và gọi executable thực nằm trong app bundle:

```text
/Applications/OffloadKit.app/Contents/MacOS/offloadkit \
  --finder-action set-source --path <path>...
```

Backend mở rộng activation parser hiện có để chấp nhận:

```text
--finder-action set-source
--finder-action set-destination
--finder-action copy
--finder-action paste
```

Sau khi parse, request được chuyển vào cùng `ExplorerAction`/`ExplorerRequest`, single-instance queue và frontend handler hiện tại. Không tạo copy engine riêng cho macOS.

Nếu app bundle không ở `/Applications` hoặc đường dẫn executable thay đổi, status báo unhealthy và workflow được reinstall từ Preferences.

## Pasteboard

Backend thêm abstraction file-list clipboard dùng chung:

- Windows: `CF_HDROP` hiện tại.
- macOS: native pasteboard file URLs (`public.file-url` / NSFilenamesPboardType-compatible input).

Copy:

- Validate/dedupe tối đa 100 paths.
- Ghi file URL list vào pasteboard.
- Không mutate composer hoặc tạo transfer.

Paste:

- Đọc file URLs.
- Validate selection qua `SourceSelection` hiện có.
- Destination phải là đúng một directory tồn tại và writable.
- Reject link/alias/reparse-like indirection khi filesystem metadata nhận diện được.
- Reset composer, ép Move và same-volume Move về `false`, rồi dùng selected-path copy pipeline hiện có.

## Workflow bundles

Bốn bundle có namespace riêng:

```text
OffloadKit Set Source.workflow
OffloadKit Set Destination.workflow
OffloadKit Copy.workflow
OffloadKit Paste.workflow
```

Mỗi bundle chứa `Contents/Info.plist` và `Contents/document.wflow`. Nội dung workflow chỉ chuyển Finder input đến executable OffloadKit; không chứa logic copy/checksum.

Install:

1. Xác minh đang chạy trên macOS.
2. Xác minh executable nằm trong `.app` dưới `/Applications`.
3. Tạo workflow trong staging cùng filesystem khi có thể.
4. Atomic replace workflow thuộc OffloadKit.
5. Read-back và validate action/executable path.
6. Rollback workflow mới nếu cài giữa chừng thất bại.
7. Refresh service cache bằng API/command hệ thống tối thiểu phù hợp; nếu Finder chưa refresh, UI hướng dẫn relaunch Finder/log out.

Uninstall:

- Chỉ xóa đúng bốn workflow OffloadKit.
- Idempotent.
- Không xóa Quick Action khác.

## Preferences

Trên macOS, General Preferences hiển thị:

```text
macOS Finder Integration
[ ] Bật Finder Quick Actions
```

- Registry/Service filesystem là source of truth.
- Loading/success/error rõ ràng.
- Nếu app chưa nằm trong `/Applications`, hiển thị yêu cầu kéo app vào Applications.
- Windows toggle không hiển thị trên Mac; Finder toggle không hiển thị trên Windows.

## Packaging

Workflow templates được bundle trong `.app` qua Tauri macOS bundle files hoặc được render từ constants đã test. Ưu tiên render từ template nhỏ để executable path luôn đúng và tránh Xcode project.

GitHub Actions phải kiểm tra trên cả Intel và Apple Silicon:

- Build `.app` và `.dmg`.
- App bundle chứa resources cần thiết nếu dùng template files.
- Rust macOS code compile.
- Workflow plist/XML parse được.

Không tuyên bố notarized/signing nếu CI chưa có Apple credentials.

## Safety

- Quick Action không copy byte; chỉ gửi request vào OffloadKit.
- Backend là trust boundary duy nhất.
- Paths từ Finder là untrusted.
- SourceSelection và destination validation giữ nguyên.
- Revalidate ngay trước selected-file open, staging/final placement.
- Paste không bao giờ bật Move.
- Clipboard rỗng/không phải file URL: không mutate state, không tạo job.
- Path chứa NUL, malformed URL hoặc ngoài filesystem local: reject.
- Không dùng shell-string interpolation với Finder path; truyền arguments an toàn bằng argv hoặc quoted workflow arguments.

## Tests

### Rust cross-platform

- Parse `--finder-action` đủ bốn action.
- Preserve spaces/Unicode.
- Reject malformed/missing paths.
- Finder request đi qua cùng queue/dedupe.
- Paste luôn ép Move false.

### macOS-gated

- Pasteboard write/read file URLs round-trip.
- Workflow render tạo plist/XML hợp lệ.
- Install → status healthy → uninstall.
- Uninstall giữ workflow không thuộc OffloadKit.
- App ngoài `/Applications` bị reject với thông báo rõ.

### Frontend

- Finder Source/Destination dùng cùng endpoint stores.
- Finder Copy không mutate composer.
- Finder Paste tạo đúng một selected-path copy.
- Preferences toggle platform-specific.

### CI/macOS smoke

- Cài workflow vào HOME tạm.
- Read-back đủ bốn action.
- Không cần thao tác Finder GUI trong CI.
- Build Intel và Apple Silicon.

### Test thật trên Mac

1. Kéo app vào Applications.
2. Mở một lần và bật Finder Integration.
3. Chuột phải selection → Quick Actions → Source/Copy.
4. Chuột phải folder → Destination/Paste.
5. Verify file/hash, Source còn nguyên, unselected absent.
6. Tắt integration và xác nhận menu biến mất.

## Acceptance criteria

1. Đủ bốn Quick Actions hoạt động trên macOS.
2. Không cần Finder Sync Extension.
3. Cài/gỡ từ Preferences và không cần admin.
4. Dùng chung selected-path copy engine và safety model.
5. Paste bắt đầu Copy an toàn, không Move/delete.
6. Full local gates pass trên Windows; macOS CI Intel/Apple Silicon pass.
7. Release có cả hai `.dmg` chứa implementation mới.

## Ngoài phạm vi

- Finder Sync badges hoặc toolbar item.
- Context menu trên vùng trống Finder.
- Apple notarization/signing credentials.
- Network/cloud file URLs.
