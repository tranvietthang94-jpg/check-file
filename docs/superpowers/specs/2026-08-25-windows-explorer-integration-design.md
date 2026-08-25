# Phase 13 — Windows Explorer Integration Design

## Mục tiêu

Cho phép người dùng thao tác OffloadKit trực tiếp từ menu chuột phải của Windows Explorer, bám sát workflow local đã xác nhận trong OffShoot 26.2.1 nhưng triển khai clean-room bằng Rust/Tauri:

1. **Đặt làm Source trong OffloadKit**.
2. **Đặt làm Destination trong OffloadKit**.
3. **Copy bằng OffloadKit**.
4. **Paste và bắt đầu transfer bằng OffloadKit**.

Phase này chỉ hỗ trợ Windows Explorer. Finder/macOS để phase riêng.

## Bằng chứng hành vi OffShoot

Source decompile đã xác nhận `ExplorerExtensionManager` đăng ký menu vào:

```text
HKCU\Software\Classes\*\shell
HKCU\Software\Classes\Directory\shell
HKCU\Software\Classes\Directory\Background\shell
```

OffShoot truyền bốn action vào app:

```text
copyToClipboard
pasteFromClipboard
setContextSource
setDestination
```

Hành vi đã xác nhận:

- `setContextSource`: nhận ổ, folder hoặc danh sách item đang chọn trong Explorer.
- `setDestination`: nhận folder làm Destination.
- `copyToClipboard`: đưa selection vào Windows File Drop Clipboard.
- `pasteFromClipboard`: lấy clipboard làm Source, folder chuột phải làm Destination, reset composer cũ và bắt đầu transfer.
- Explorer integration có toggle trong Settings và đăng ký dưới HKCU, không cần Administrator.

OffloadKit chỉ sao chép **hành vi công khai**, không sao chép code, branding, asset, license/DRM hay cloud workflow.

## Quyết định kiến trúc

### Registry verbs thay vì COM shell extension

OffloadKit dùng Windows Registry verbs gọi executable:

```text
OffloadKit.exe --explorer-action set-source --path "%V"
OffloadKit.exe --explorer-action set-destination --path "%V"
OffloadKit.exe --explorer-action copy --path "%V"
OffloadKit.exe --explorer-action paste --path "%V"
```

Lý do:

- Gần với kiến trúc thực tế của OffShoot.
- Không inject DLL vào `explorer.exe`.
- Lỗi OffloadKit không làm crash Explorer.
- Đăng ký theo user, không cần quyền admin.
- Dễ test và gỡ sạch.

Giới hạn chấp nhận: Windows 11 có thể đặt các verb registry truyền thống trong **Show more options**. Native `IExplorerCommand`/COM không nằm trong phase này.

## Phân chia triển khai

### Phase 13A — Source/Destination integration

- Parse command-line Explorer action.
- Single-instance forwarding.
- Đưa folder/ổ vào Source hoặc Destination.
- Focus cửa sổ và hiển thị lỗi rõ ràng.
- Preferences toggle cài/gỡ Explorer integration.
- Registry install/read-back/uninstall.

### Phase 13B — Copy/Paste transfer workflow

- Đọc/ghi Windows File Drop Clipboard.
- Hỗ trợ selection gồm nhiều file/folder.
- Paste reset composer, tạo Source selection + Destination và bắt đầu transfer.
- Mở rộng copy engine để copy đúng selection thay vì quét toàn bộ root.
- Giữ toàn bộ safety guarantees của copy/move hiện tại.

Cả 13A và 13B thuộc Phase 13 nhưng phải commit và verify riêng.

## Windows Explorer menu

### Folder hoặc ổ đĩa

```text
OffloadKit
├── Đặt làm Source
├── Đặt làm Destination
├── Copy bằng OffloadKit
└── Paste và bắt đầu transfer
```

### File hoặc selection

```text
OffloadKit
├── Đặt làm Source
└── Copy bằng OffloadKit
```

### Background của folder

```text
OffloadKit
├── Đặt folder hiện tại làm Source
├── Đặt folder hiện tại làm Destination
└── Paste và bắt đầu transfer
```

Registry keys phải dùng namespace riêng `OffloadKit.*`; uninstall chỉ được xóa key của OffloadKit.

## Activation và single instance

### App chưa chạy

1. Explorer khởi chạy `OffloadKit.exe` với action/path.
2. Backend parse và validate request.
3. Cửa sổ app mở.
4. Frontend nhận event khi stores đã hydrate.
5. Action được thực hiện đúng một lần.

### App đang chạy

1. Instance thứ hai chuyển payload sang instance chính.
2. Instance thứ hai thoát.
3. Instance chính focus cửa sổ với action Source, Destination hoặc Paste.
4. Copy được phép hoàn thành clipboard action mà không bắt buộc focus app.

Payload nội bộ:

```rust
pub enum ExplorerAction {
    SetSource,
    SetDestination,
    Copy,
    Paste,
}

pub struct ExplorerRequest {
    pub action: ExplorerAction,
    pub paths: Vec<PathBuf>,
}
```

Mỗi request có ID để frontend dedupe và xử lý đúng một lần.

## Path và selection model

Endpoint hiện phụ thuộc `diskId`, không đủ cho folder/file tùy ý. Phase 13 thêm path-based endpoint tối thiểu:

```ts
interface Endpoint {
  id: string;
  diskId: string | null;
  label: string;
  path: string;
  selectedPaths?: string[];
  isAutoLabel: boolean;
}
```

Quy tắc:

- Folder đơn dùng `path` và không cần `selectedPaths`.
- Multi-selection dùng common root + `selectedPaths`.
- Nếu path thuộc disk đã enumerate, gắn `diskId`; nếu không thì `diskId = null`.
- Endpoint không có disk vẫn copy được nhưng không được rename/eject volume.
- Không dùng đường dẫn giả hoặc data giả.

## Multi-selection

OffShoot đọc selection đang active qua Explorer automation. OffloadKit bản đầu dùng registry invocation aggregation:

- Windows gọi command một hoặc nhiều lần cho selection.
- Backend gom request cùng action trong cửa sổ debounce ngắn.
- Dedupe path không phân biệt hoa/thường trên Windows.
- Giới hạn tối đa 100 item giống hành vi quan sát được của OffShoot.
- Nếu selection không thu được đầy đủ bằng registry verb trên một phiên bản Windows cụ thể, fallback là path `%V` đang kích hoạt; không dùng COM automation trong Phase 13 trừ khi integration test chứng minh registry không đủ.

## Copy/Paste semantics

### Copy bằng OffloadKit

- Validate các path tồn tại.
- Dedupe selection.
- Ghi Windows File Drop Clipboard.
- Không copy byte và không tạo transfer lúc này.
- Không thay đổi Source/Destination đang có.

### Paste và bắt đầu transfer

1. Đọc File Drop Clipboard.
2. Nếu clipboard rỗng/không hợp lệ: báo lỗi, không thay state.
3. Validate Destination là folder tồn tại và writable.
4. Chuẩn hóa selection và loại path con dư thừa nếu folder cha đã được chọn.
5. Kiểm tra Source/Destination không trùng hoặc chồng lấn.
6. Reset Source/Destination composer cũ.
7. Tạo Source selection và Destination.
8. Bắt đầu transfer dùng settings hiện tại.
9. Không bật Move ngầm; Paste mặc định là copy an toàn.

## Copy engine selected-path support

Backend thêm:

```rust
pub struct SourceSelection {
    pub common_root: PathBuf,
    pub selected_paths: Vec<PathBuf>,
}
```

Quy tắc fail-closed:

- Mọi selected path phải nằm trong `common_root`.
- Reject `..`, absolute relative components, symlink, junction và reparse point.
- Folder được duyệt bằng `WalkDir` với follow-links tắt.
- Nếu chọn folder cha và item con, chỉ giữ folder cha.
- Giữ relative layout tính từ `common_root`.
- Staging, checksum, retry, missing-file detection, MHL và logs dùng pipeline hiện có.
- Move không nằm trong Explorer Paste của phase này.

## Registry management

Backend commands:

```text
install_explorer_integration
uninstall_explorer_integration
explorer_integration_status
```

Install:

- Lấy executable hiện tại từ `current_exe()`.
- Ghi key dưới HKCU.
- Quote executable và path đúng chuẩn Windows.
- Ghi icon từ executable.
- Read-back toàn bộ command sau khi ghi.
- Nếu một key lỗi, rollback các key đã tạo trong lần install đó.

Uninstall:

- Chỉ xóa `OffloadKit.*` keys.
- Idempotent: key không tồn tại vẫn thành công.
- Không sửa menu của app khác.

## Preferences

General Preferences thêm:

```text
Windows Explorer Integration
[ ] Bật menu chuột phải OffloadKit
```

- Chỉ hiển thị trên Windows.
- UI đọc trạng thái thực từ Registry, không chỉ tin persisted setting.
- Bật/tắt có loading, success và error state.
- Nếu executable path đổi sau upgrade, bật lại hoặc startup health-check sửa command path.

## Files dự kiến

### Tạo

```text
src-tauri/src/explorer_integration.rs
src-tauri/src/source_selection.rs
src/state/explorerActionStore.ts
e2e/explorer-actions.spec.ts
```

### Sửa

```text
src-tauri/Cargo.toml
src-tauri/src/lib.rs
src-tauri/src/commands.rs
src-tauri/src/copy_engine.rs
src-tauri/src/cascade.rs
src-tauri/tauri.conf.json
src/types/disk.ts
src/types/job.ts
src/state/disksStore.ts
src/lib/tauri.ts
src/App.tsx
src/components/preferences/GeneralPreferences.tsx
.github/workflows/build.yml
README.md
```

Dependencies chỉ được thêm nếu Rust stdlib/Win32 FFI hiện có không đủ. Ưu tiên Windows Registry API trực tiếp hoặc crate đã có; không thêm shell-extension framework.

## Error handling

- Action không hợp lệ: bỏ qua và log, không mutate state.
- Path không tồn tại: hiển thị error trong app.
- Path file dùng làm Destination: reject.
- Clipboard không phải File Drop: reject.
- Destination không writable: reject trước khi tạo job.
- App nhận hai request trùng ID: xử lý một lần.
- Registry install lỗi giữa chừng: rollback.
- Instance forwarding lỗi: instance thứ hai mở app và hiển thị lỗi thay vì mất request.

## Testing

### Rust unit/integration

- Parse bốn action.
- Quote path có spaces, `&`, Unicode tiếng Việt và dấu ngoặc kép.
- Reject action/path malformed.
- Registry install → read-back → uninstall trên hive test hoặc namespace tạm.
- Uninstall không xóa unrelated key.
- Request dedupe/debounce.
- Common-root calculation.
- Parent selection removes nested child.
- Reject path outside root.
- Reject symlink/junction/reparse point.
- Paste empty clipboard no-op.
- Paste never enables Move.

### Frontend/Playwright

- Set Source action tạo Source path-based.
- Set Destination action tạo Destination.
- Duplicate action không tạo endpoint lần hai.
- Invalid action hiển thị error.
- Paste tạo group/job với settings hiện tại.
- Preferences toggle có status/error.

### Windows integration thật

Dùng folder test, không dùng dữ liệu sản xuất để Paste:

```text
D:\MATERIALS
F:\BACKUP HXM
F:\Adobe Premiere Pro Auto-Save
```

- Chuột phải folder → Set Source.
- Chuột phải folder → Set Destination.
- Chọn nhiều file nhỏ test → Copy.
- Chuột phải folder test rỗng → Paste.
- Verify hash Source/Destination.
- Xác nhận Source gốc còn nguyên.
- Tắt integration → menu biến mất.
- Uninstall build → registry sạch.

## Acceptance criteria

Phase 13 hoàn thành khi:

1. Đủ bốn lệnh Explorer hoạt động trên Windows 11.
2. Source/Destination được cập nhật đúng trong instance đang chạy hoặc mới mở.
3. Copy/Paste hỗ trợ selection thực và Paste bắt đầu copy an toàn.
4. Không có Move/delete ngầm.
5. Path/symlink/junction protections giữ nguyên.
6. Registry install/uninstall fail-closed và không cần admin.
7. Full build, Playwright và Rust tests pass.
8. Integration thật pass trên folder test.
9. Installer mới tự mang đầy đủ integration và gỡ sạch.

## Ngoài phạm vi

- Finder/macOS integration.
- Windows 11 modern context menu COM `IExplorerCommand`.
- Network/cloud clipboard sources.
- License/DRM, telemetry hoặc proprietary OffShoot code.
- Tự động Move từ Paste.
