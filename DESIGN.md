# DESIGN.md — OffloadKit UI Design Brief (cho Google Stitch)

> Tài liệu này mô tả **giao diện** của OffloadKit — một desktop app (Tauri + React,
> Windows) dùng để offload/copy media từ thẻ nhớ hoặc ổ rời sang một hoặc nhiều ổ đích,
> có xác minh checksum, ghi MHL, đổi tên/tổ chức file theo template, và xuất báo cáo.
> Toàn bộ logic backend (Rust/Tauri command, thuật toán hash, ghi file...) được **bỏ qua**
> — tài liệu chỉ tập trung vào những gì người dùng nhìn thấy và thao tác.

Đối tượng dùng app: DIT / camera assistant trên trường quay, cần theo dõi tiến trình
copy nhiều thẻ cùng lúc và tin tưởng dữ liệu đã được sao lưu toàn vẹn. Ưu tiên: **rõ
ràng, mật độ thông tin cao, thao tác nhanh**, không cần màu mè.

---

## 1. Phong cách thiết kế tổng thể

> **Cập nhật:** phiên bản Google Stitch tạo ra (`new ui/DESIGN.md` +
> `new ui/offloadkit_localized_design_document_vn.md`) đã được rà soát và merge vào
> đây. Phần **style system** (màu, font, bo góc, elevation) của Stitch được giữ lại vì
> khớp tốt với brief ban đầu và đã được chốt lại số liệu chính xác. Phần **mô tả chức
> năng** của Stitch có nhiều chỗ hallucination (bịa tính năng không có trong code) —
> đã **không** đưa vào bên dưới; toàn bộ được liệt kê riêng ở [Phụ lục — mục 9](#9-phụ-lục--rà-soát-bản-thiết-kế-stitch-tạo-new-ui) để tránh nhầm là đã tồn tại.

- **Theme:** chỉ Dark mode, phong cách **"High-Contrast Brutalist"** — dành cho công cụ
  kỹ thuật (DIT tool), không phải app tiêu dùng. Không gradient, không đổ bóng mềm,
  không chi tiết trang trí.
- **Màu nền/bề mặt:**
  - Nền gốc (Level 0): đen tuyền `#000000` (hoặc `#131313` nếu cần phân biệt với
    thanh title bar hệ điều hành) — mục tiêu là giảm chói mắt khi làm việc on-set.
  - Panel (Level 1): than chì đậm `#111111`–`#1c1b1b`, viền `1px solid` trắng mờ
    (~30% opacity) hoặc `#3a3939`.
  - Level 2 (active/hover/focus): sáng hơn 1 bậc, hoặc viền xanh lá 1px đặc.
  - **Không dùng đổ bóng (shadow)** để tạo chiều sâu — chỉ dùng viền 1px + nền đen để
    "cắt" lớp trên (modal, dropdown) khỏi lớp dưới.
- **Mã màu chức năng (chốt số liệu cụ thể, dùng xuyên suốt toàn app):**

| Vai trò | Hex | Dùng cho |
|---|---|---|
| Primary / border / label chính | `#FFFFFF` | Viền 1px mặc định, icon, text chính |
| Success / Action / Verified | **`#19ED60`** (xanh lá rực) | Nút hành động chính ("Start Transfer"), progress bar khi copying/complete, badge "Verified" |
| Error / Failed | `#FF3B30` (đỏ) hoặc token M3 `#FFB4AB` | File lỗi, mismatch, resume bị chặn |
| Cảnh báo (nhẹ hơn error) | Cam/Vàng | Broken media, size mismatch, move-delete failed, renamed |
| Queued / trung tính | Xám (`on-surface-variant` ~`#c4c7c8`) | Job chưa bắt đầu, text phụ |

  > Ghi chú kỹ thuật: file gốc của Stitch có 2 chỗ không khớp nhau — phần mô tả prose
  > ghi xanh lá là `#19ED60`, nhưng khối token YAML lại định nghĩa `secondary:
  > #84ff92` / `secondary-container: #03e85c` (không trùng giá trị nào). Khi code,
  > **dùng đúng một giá trị `#19ED60`** làm nguồn chân lý duy nhất cho màu xanh lá;
  > bỏ qua các token `secondary*`/`tertiary*` còn lại trong file YAML — đó là token
  > Material-3 tự sinh, không được nhắc tới ở phần mô tả component nào cả.
- **Typography:** 2 họ font, dùng đúng vai trò:
  - **Hanken Grotesk** (sans-serif) — headline, nhãn, text thường. Thang chữ gợi ý:
    `headline-lg` 32px/700, `headline-md` 20px/600, `body-lg` 16px/400, `body-sm`
    14px/400.
  - **JetBrains Mono** (monospace) — **bắt buộc** cho mọi dữ liệu kỹ thuật: đường dẫn
    (path), tên file, checksum/hash, timecode, token template (`{YYYY}`,
    `{Counter}`...). Thang chữ: `mono-data` 13px/500, `label-caps` 11px/700 viết
    HOA + letter-spacing rộng (dùng cho label phía trên input, tiêu đề section).
- **Bo góc:** **0px** ở mọi nơi (button, input, card, modal) — phong cách "công cụ",
  không bo tròn. Việc này còn giúp viền 1px thẳng hàng pixel-perfect trên màn hình
  độ phân giải cao.
- **Mật độ:** dense/compact — nhiều panel nhỏ hiển thị cùng lúc trên một màn hình
  (giống dashboard chuyên nghiệp kiểu DaVinci Resolve / Silverstack, không phải app
  tiêu dùng thoáng đãng). Grid 12 cột trên desktop, gộp về 1 cột trên mobile; nhịp
  spacing theo baseline 4px (`xs 4 · sm 8 · md 16 · lg 24 · xl 48`, gutter 12px).
- **Component cụ thể:**
  - *Button*: hình chữ nhật 0 góc bo. Primary = nền đen + viền trắng 1px + chữ trắng.
    Action/Success = nền xanh lá `#19ED60` + chữ đen đậm (dùng cho "Start Transfer",
    "Generate Report"...). Ghost = nền trong suốt + viền trắng mờ 30%.
  - *Input*: viền trắng 1px, font JetBrains Mono, label `label-caps` nằm phía trên.
    Focus = viền chuyển sang 100% trắng hoặc xanh lá `#19ED60`.
  - *Bảng dữ liệu*: đường kẻ ngang 1px, header nền than chì đậm hơn data row, số liệu
    dùng `mono-data`.
  - *Progress bar*: phẳng, không bo đầu, track than chì đậm, fill xanh lá `#19ED60`
    khi copying/complete, đổi sang đỏ khi có lỗi.
  - *Status chip*: hình chữ nhật nhỏ viền 1px, không có nền trừ khi là trạng thái
    "Active" (xanh lá) hoặc "Error" (đỏ); chữ dùng JetBrains Mono.
- Mọi control bị **disabled** đều cần **tooltip giải thích lý do** (đây là hành vi hiện
  có, rất quan trọng cho UX — ví dụ nút Eject bị mờ kèm tooltip "Wait for active
  transfers on this disk to finish").

---

## 2. Kiến trúc màn hình (Information Architecture)

Điều hướng chính gồm **2 màn hình lớn**:

1. **Home** (mặc định khi mở app) — nơi thao tác offload: chọn ổ, cấu hình nhanh, xem
   tiến trình, xem log.
2. **Settings** — cấu hình chi tiết: Verification, Queueing, Organize (rename/tổ chức
   file), Presets.

Ngoài ra có **3 màn hình/modal phụ**, mở ra từ Home, cần thiết kế đầy đủ vì chứa dữ
liệu bắt buộc của app:

3. **Media Browser** (modal) — xem trước clip trong một thư mục nguồn.
4. **Verify MHL** (panel/tab riêng) — kiểm tra lại checksum của file `.mhl` đã có.
5. **Reports** (panel/tab riêng) — chọn các log đã copy xong để xuất báo cáo PDF/HTML.

Gợi ý bố cục nav cho Stitch: sidebar hoặc top-tab với 2 mục chính **Home | Settings**,
và các nút/icon phụ trong Home để mở Media Browser / Verify MHL / Reports (dạng
drawer, modal, hoặc tab phụ cùng cấp) — miễn giữ đúng nội dung/chức năng liệt kê bên dưới.

---

## 3. Màn hình HOME (Dashboard chính)

Home là một dashboard nhiều khối (multi-panel), người dùng cuộn dọc hoặc chia cột.
Gồm các khối theo đúng thứ tự luồng thao tác:

### 3.1 Disks Panel — danh sách toàn bộ ổ đĩa phát hiện được

Mỗi **Disk Card** hiển thị:
- Tên ổ đĩa (`disk.name`)
- Mount point + dung lượng: `"D:\ · 120.5 GB free of 500 GB · removable"`
- Tag "removable" nếu là ổ rời (thẻ nhớ/USB)
- 4 nút hành động trên mỗi card:
  - **+ Source** (disable nếu ổ đã là Source)
  - **+ Destination** (disable nếu ổ đã là Destination)
  - **Eject** (chỉ hiện với ổ removable; disable + tooltip khi ổ đang có job
    queued/copying; trạng thái "Ejecting…" khi đang chạy; hiển thị lỗi eject dạng text đỏ nhỏ nếu thất bại)
  - **Hide** (disable + tooltip nếu ổ đang được gán làm Source/Destination)
- **Trạng thái rỗng:** "No volumes detected." (chưa cắm ổ nào) hoặc "Every detected
  drive is hidden -- unhide one below." (tất cả bị ẩn)
- Khu vực **Hidden drives** thu gọn: nút "Show/Hide hidden drives (N)", danh sách ổ đã
  ẩn kèm nút **Unhide** từng cái.

### 3.2 Source List — **bắt buộc**

Danh sách các ổ đã gán làm nguồn. Mỗi **Source Card**:
- Tên ổ + mount point (hiển thị `"(unplugged)"` nếu ổ đã bị rút khỏi máy)
- Ô nhập **Label** (viền xanh dương nếu người dùng tự gõ, viền trắng nếu là Auto
  Label tự sinh, viền xám nếu rỗng)
- Nút **Browse** — mở Media Browser để xem trước nội dung ổ này
- Nút **Remove** (chữ đỏ)
- Ô nhập **Folder path** dạng monospace (đường dẫn đầy đủ tới thư mục nguồn)
- **Trạng thái rỗng:** "None assigned yet."

### 3.3 Destination List — **bắt buộc**

Giống Source Card nhưng **không có nút Browse**. Có thể có nhiều Destination cùng lúc
(hỗ trợ copy song song ra nhiều ổ).

### 3.4 Build Transfer (Group Composer)

Form để tạo một lượt transfer mới:
- Radio chọn **1 Source** (danh sách các Source đã gán)
- Checkbox chọn **nhiều Destination** — thứ tự tick chọn = thứ tự cascade (hiển thị
  `(primary)` cho đích đầu tiên, `(#2)`, `(#3)`... cho các đích sau khi Mode = Cascade)
- Chọn **Mode**:
  - *Parallel*: "Each destination copies from the source independently"
  - *Cascade*: "Source copies to the primary destination first, then relays to the rest"
- Checkbox **Move (delete source after verified copy)** — chỉ bật được khi đúng 1
  destination được chọn **và** Verification Mode ≠ Transfer; nếu không đủ điều kiện,
  hiển thị mờ kèm dòng giải thích tương ứng (khác 1 đích / cần bật verification)
- Nút **Start Transfer** (disable nếu chưa chọn đủ source + destination)
- **Trạng thái rỗng:** "Assign at least one Source and one Destination to build a transfer."

### 3.5 Quick Settings (rút gọn ngay trên Home)

- 3 lựa chọn **Verification Mode** (radio card, mỗi card có tiêu đề + mô tả phụ):
  - Transfer — "Size check only, fastest"
  - Source — "Hash source while copying"
  - Source & Destination — "Hash both, compare (safest)"
- Dropdown **Checksum Algorithm**: XXH64 / MD5 / SHA-1 (disable khi Verification =
  Transfer)

*(Bộ đầy đủ các setting khác nằm ở màn hình Settings — xem mục 4.)*

### 3.6 Transfers — Progress tổng & chi tiết — **bắt buộc**

Đây là khối quan trọng nhất, hiển thị các nhóm transfer (Group) đang chạy hoặc đã chạy
trong phiên hiện tại.

**Cấu trúc phân cấp:** 1 Transfer Group → nhiều Job (1 job = 1 cặp source→destination).

Mỗi **Group Card**:
- Header: `"{sourceLabel} → {destinationLabel1}, {destinationLabel2}..."` + tag chế độ
  (Parallel/Cascade) + nút **Cancel Group** (chỉ hiện khi còn job đang active)
- **Progress bar TỔNG (overall)** — gợi ý bổ sung cho thiết kế: 1 thanh progress lớn ở
  đầu group, tổng hợp `Σ bytesCopied / Σ totalBytes` của tất cả job active trong group,
  kèm % tổng và tốc độ cộng dồn. (Hiện dữ liệu per-job đã có sẵn để tính; đây là thanh
  progress "tổng" mà theo yêu cầu thiết kế cần có, đặt phía trên danh sách job chi tiết.)
- Danh sách **Job Row** (progress chi tiết từng đích), mỗi row gồm:
  - Tên đích (kèm nhãn `relay →` nếu job này là hop 2 — tức được relay từ đích chính
    trong chế độ Cascade)
  - Góc phải: nút **Resume** (chỉ hiện khi job cancelled, hoặc complete nhưng có file
    lỗi) / nút **Cancel** (khi đang active) / text trạng thái viết hoa chữ đầu khi đã
    kết thúc (Complete/Cancelled)
  - **Progress bar chi tiết** (1 job): % = bytesCopied/totalBytes, màu theo trạng thái
    job (xám/xanh dương/xanh lá/cam)
  - Dòng phụ: tên file đang copy hiện tại (`currentFile`, hoặc "—" nếu rỗng) + bên
    phải: `"{bytesCopied} / {totalBytes}"`, thêm `"· {tốc độ} MB/s"` khi đang copy
  - **Banner cảnh báo Broken Media** (nền cam mờ, border cam) khi có file 0-byte phát
    hiện trên nguồn: `"⚠ N broken (0-byte) file(s) found on the source — likely a card
    that dropped out mid-recording."` kèm 2 nút: **Continue Anyway** / **Cancel Job**
  - Dòng cảnh báo đỏ nếu **Resume bị chặn**: `"⚠ Source disk has changed since this
    transfer started -- reconnect the original source before resuming."`
  - Các dòng kết quả (chỉ hiện khi có dữ liệu tương ứng, mỗi dòng 1 màu riêng):
    - Xanh lá: `"N file(s) verified (source = destination, XXH64)"` hoặc `"N file(s)
      hashed (XXH64)"` tùy verification mode
    - Xám: `"N file(s) skipped (already offloaded)"`
    - Vàng: `"N file(s) renamed (name already used by a different file)"`
    - Đỏ: `"N file(s) failed — {thông báo lỗi đầu tiên}"`
    - Xám: `"N file(s) moved (source removed after verified copy)"`
    - Cam: `"N file(s) copied but the source could not be removed — {lỗi}"`
    - Cam: `"N broken (0-byte) file(s) were found on the source"` (khi đã xử lý xong
      broken-media, không còn ở trạng thái chờ quyết định)
  - Khi Cascade và đích thứ 2 chưa bắt đầu: dòng xám `"Waiting to relay…"`
- **Trạng thái rỗng:** "No transfers yet."

### 3.7 Transfer Log / bảng MD5 Checksum — **bắt buộc**

Bảng/lưới các phiên transfer đã hoàn tất (lịch sử), dạng thẻ dàn lưới 1–3 cột tùy độ
rộng màn hình. Mỗi **Log Entry Card**:
- Tên nguồn (`sourceName`) + tag cam "Stopped" nếu job đó đã bị hủy
- Thời điểm hoàn tất (định dạng ngày giờ địa phương), căn phải
- Dòng đường dẫn: `"{source} → {destination}"` (monospace, rút gọn kèm tooltip đầy đủ)
- Dòng thống kê (các mục chỉ hiện khi > 0):
  - `"{filesCopied} file(s) · {dung lượng}"`
  - `"{N} skipped"`, `"{N} renamed"`, `"{N} failed"` (đỏ), `"{N} moved"`, `"{N}
    move-delete failed"` (cam)
  - `"MHL: {tên_file.mhl}"` nếu job có ghi file MHL
- Nút **View clips** — mở Media Browser tại đường dẫn đích
- Nút **Refresh** ở header toàn bảng
- **Trạng thái rỗng:** "No completed transfers yet."
- Trạng thái loading: "Loading…"; trạng thái lỗi tải log: text đỏ

> **Ghi chú thiết kế cho bảng checksum chi tiết:** dữ liệu mỗi log đã có sẵn danh sách
> file kèm checksum (`verifiedFiles[]`: path, checksum, algorithm), nhưng hiện chỉ hiển
> thị dạng đếm số lượng. Khuyến nghị Stitch thiết kế thêm **view chi tiết mở rộng** khi
> click vào 1 log entry: một bảng liệt kê từng file với các cột — **Tên file | Kích
> thước | Thuật toán | Giá trị checksum (mono, có thể copy) | Trạng thái** (Verified /
> Skipped / Renamed / Failed), phục vụ đúng yêu cầu "bảng Log/MD5 Checksum".

---

## 4. Màn hình SETTINGS

Chia thành các nhóm rõ ràng (có thể dùng section/card hoặc sub-tab):

### 4.1 Verification
- 3 radio card Verification Mode (Transfer/Source/Source & Destination) — như mục 3.5
  nhưng đầy đủ mô tả
- Dropdown Checksum Algorithm (XXH64/MD5/SHA-1), disable khi mode = Transfer
- Checkbox **Prevent sleep during transfer**
- Checkbox **Desktop notifications**

### 4.2 Queueing
4 lựa chọn radio card:
| Mode | Mô tả |
|---|---|
| Off | Every transfer starts immediately |
| Single Source | One source's destinations at a time; next source auto-starts |
| Single Destination | One destination at a time per source |
| Single Transfer | One job at a time, app-wide |

### 4.3 Organize (đổi tên & tổ chức file khi copy)

- Ô nhập **Rename template** (monospace, placeholder "Keep original filename…")
- Ô nhập **Folder template** (monospace, disable khi bật Flatten)
- Dòng **Preview** (monospace, nền tối) hiển thị kết quả path render thử ngay khi gõ
- Dòng chú thích các token khả dụng: `{Source Name} {Counter} {YYYY}{YY}{MM}{DD}
  {hh}{mm}{ss} · {Filename} {File Counter} {File Extension} {File YYYY}.. ·
  {Content YYYY}..`
- Số **{Counter} padding** (input số, 1–8)
- **Selective copy**: radio "Do not copy" (exclude) / "Copy only" (include) + ô nhập
  danh sách pattern cách nhau bởi dấu phẩy (vd `.xml, proxy, .tmp`)
- Checkbox **Skip Modification Date Check** — "Duplicate Detection compares name + size
  only -- for workflows where a file's modified time can't be trusted to still match."
- Checkbox **Auto-Continue on Broken Media** — "Skips the alert when a 0-byte file is
  found on the source and copies anyway. Off by default, so a dropped card gets
  flagged before anything is copied."
- Checkbox **Ignore bundle folder** + khi bật: ô nhập tên thư mục (vd "PRIVATE") + số
  MB tối đa
- Checkbox **Flatten (discard original subfolders)**
- Checkbox **Ignore empty folders** (disable khi Flatten bật)
- Ô nhập **{Content *} excludes extensions** (vd `.xml`)
- **Shoot date** ({YYYY}{MM}{DD}): radio "Follow system clock" (automatic, kèm
  checkbox "Roll over at 4am") / "Set manually" (date picker + nút "Now" để quay lại
  automatic)
- **Elements (custom tokens)**: danh sách token tự định nghĩa, mỗi dòng gồm tên token
  (dạng `{TênToken}`), ô nhập giá trị cho job hiện tại, nút xóa (×); form thêm token
  mới (tên + nút "+ Add"); nút "Clear values" xóa hết giá trị đã nhập; ghi chú hướng
  dẫn cách dùng token trong Rename/Folder template
- **Auto Label new sources**: checkbox bật/tắt; khi bật hiện thêm: ô nhập template
  (vd `{Source Name}_{Counter}`), dòng Preview render thử, số "Start at" (counter bắt
  đầu), số "Padding"

### 4.4 Presets
- Danh sách preset đã lưu, mỗi dòng: tên preset + nút **Load** + nút **Delete** (đỏ)
- Trạng thái rỗng: "No presets saved yet."; loading: "Loading…"; lỗi: text đỏ
- Form lưu preset mới: ô nhập tên + nút "Save current" (lưu toàn bộ Verification +
  Organize hiện tại thành 1 preset)

---

## 5. Màn hình/Modal phụ

### 5.1 Media Browser (modal "Browse Source")
- Header: tiêu đề "Browse Source" + đường dẫn folder đang xem + bên phải: đếm số file
  (`"Scanning… {N}"` khi đang quét hoặc `"{N} file(s)"` khi xong) + nút **Close**
- Lưới thumbnail (2–6 cột tùy độ rộng), mỗi ô:
  - Vùng thumbnail tỉ lệ 16:9 (ảnh preview nếu có, hoặc nhãn loại file `video/audio/
    photo/other` nếu không có thumbnail)
  - Tên file (đường dẫn, rút gọn)
  - Dung lượng file
  - Dòng metadata tùy loại:
    - **Video:** độ phân giải (WxH) · codec · frame rate · thời lượng · timecode
    - **Audio:** codec · sample rate (kHz) · số kênh · thời lượng
    - **Photo:** camera model · lens · focal length · aperture · shutter speed · ISO
- Trạng thái rỗng: "No files found." (khi quét xong mà không có file nào)

### 5.2 Verify MHL
- Mô tả ngắn: "Re-checks an existing .mhl file's recorded checksums against the real
  files on disk -- no transfer required."
- Radio chọn chế độ: "Single .mhl file" / "All .mhl files in a folder"
- Ô nhập path (monospace) + nút **Verify** (trạng thái "Verifying…" khi đang chạy)
- Kết quả: 1 hoặc nhiều **Report Card**, mỗi card:
  - Đường dẫn file .mhl + bên phải: `"{N} OK"` (xanh lá, không có vấn đề) hoặc `"{N}
    issue(s)"` (đỏ)
  - Danh sách từng file: tên file tương đối + trạng thái màu tương ứng:
    | Trạng thái | Màu |
    |---|---|
    | Verified | Xanh lá |
    | Checksum mismatch | Đỏ |
    | Missing | Đỏ |
    | Size mismatch | Cam |
    | No checksum recorded | Xám |
  - Trạng thái rỗng riêng: "No .mhl files found there."

### 5.3 Reports
- Danh sách checkbox chọn các Transfer Log entry để đưa vào báo cáo (tên nguồn + thời
  gian hoàn tất)
- Ô nhập **Title** (mặc định placeholder "Transfer Report")
- Upload **Logo** (optional): input file ảnh, giới hạn 5MB (báo lỗi nếu vượt), preview
  logo nhỏ kèm tên file, nút "Remove"
- Ô nhập **Notes** (textarea)
- Checkbox **Include clip thumbnails** — "slower -- re-reads files at their destination"
- Nút **Generate Report** (disable khi chưa chọn log nào; text "Generating…" khi đang
  chạy)
- Kết quả: text hiển thị đường dẫn file đã lưu + ghi chú "opened in your browser. Use
  Print → Save as PDF for a PDF copy."; hoặc lỗi màu đỏ
- Trạng thái rỗng: "No completed transfers yet -- Reports summarize one or more
  Transfer Log entries."

---

## 6. Bảng tổng hợp trạng thái UI bắt buộc

| Trạng thái | Áp dụng ở đâu | Thể hiện thị giác |
|---|---|---|
| **Idle** | Toàn app khi chưa có source/destination hoặc chưa start transfer | Panel hiện text rỗng hướng dẫn ("None assigned yet.", "No transfers yet."...); nút Start Transfer bị disable |
| **Copying** | Job đang chạy | Progress bar xanh dương chuyển động; hiện currentFile + tốc độ; nút Cancel thay cho text trạng thái |
| **Success** | Job hoàn tất, không lỗi | Progress bar 100% xanh lá; text trạng thái "complete"; các dòng tổng kết (verified/hashed, skipped, renamed, moved) |
| **Error** | Job có file lỗi / resume bị chặn / MHL mismatch/missing | Text/badge đỏ; dòng lỗi kèm nút Resume xuất hiện lại |
| **Warning** | Broken media alert, size mismatch, move-delete failed, renamed file | Banner hoặc text màu cam/vàng, không chặn luồng chính nhưng cần chú ý |
| **Cancelled** | Job bị người dùng hủy | Progress bar dừng màu cam; text "cancelled"; nút Resume xuất hiện |
| **Loading/Busy** | Đang tải preset/log, đang verify MHL, đang generate report, đang eject | Text "Loading…" / "Verifying…" / "Generating…" / "Ejecting…"; nút hành động disable trong lúc chờ |
| **Disabled (có điều kiện)** | Rất nhiều control phụ thuộc điều kiện khác | Control mờ (opacity thấp) + tooltip giải thích lý do bị khóa |

---

## 7. Tham chiếu nhanh: các trường dữ liệu chính (để thiết kế đúng nội dung hiển thị)

**DiskInfo:** name, mountPoint, totalBytes, availableBytes, isRemovable, fileSystem

**Endpoint (Source/Destination):** diskId, label, path, isAutoLabel

**TransferJob:** status (queued/copying/complete/cancelled), currentFile, bytesCopied,
totalBytes, filesCopied, totalFiles, bytesPerSec, failedFiles[], verifiedFiles[]
(path+checksum+algorithm), skippedFiles[], renamedFiles[] (originalPath→renamedTo),
deletedSourceFiles[], moveDeleteFailed[], brokenMediaFiles[], pendingBrokenMedia,
resumeBlockedReason, hop (1=gốc, 2=relay cascade)

**TransferGroup:** mode (parallel/cascade), sourceLabel, destinationLabels[], jobIds[]

**TransferLogEntry:** sourceName, source, destination, startedAt, finishedAt,
filesCopied, bytesCopied, failedFiles/verifiedFiles/skippedFiles/renamedFiles,
deletedSourceFiles, moveDeleteFailed, mhlPath, cancelled

**MhlVerifyReport:** mhlPath, results[] (relativePath + status)

**Preset:** name, verificationMode, checksumAlgorithm, organize settings đầy đủ

**MediaEntry:** path, size, kind (video/audio/photo/other), metadata (tùy loại),
thumbnailBase64

---

## 8. Ghi chú khi đưa vào Google Stitch

- Ưu tiên mô tả **2 màn hình Home & Settings** làm khung chính khi viết prompt cho
  Stitch; mô tả 3 screen phụ (Media Browser, Verify MHL, Reports) như "modal/tab mở từ
  Home" để Stitch không tách chúng thành app riêng.
- Vì đây là công cụ chuyên dụng (professional tool), nên nhấn mạnh với Stitch: **dark
  theme, dense layout, monospace cho path/checksum, color-coding trạng thái nhất
  quán** — tránh phong cách "app tiêu dùng" bo tròn lớn/nhiều khoảng trắng.
- Có thể tách prompt theo từng phần ở trên (3.1 → 3.7, rồi 4.1 → 4.4, rồi 5.1 → 5.3)
  nếu Stitch giới hạn độ dài mỗi lần generate, vì tài liệu đã được chia theo đúng ranh
  giới component/card.

---

## 9. Phụ lục — Rà soát bản thiết kế Stitch tạo (`new ui/`)

Sau khi đưa bản DESIGN.md ở trên vào Google Stitch, Stitch trả về 2 file trong
`new ui/`: một style guide (`DESIGN.md` — đã merge vào mục 1 ở trên) và một tài liệu
mô tả màn hình bằng tiếng Việt (`offloadkit_localized_design_document_vn.md`). Đối
chiếu tài liệu thứ hai với source code thật, phần **mô tả chức năng** có nhiều chi
tiết Stitch tự suy diễn/bịa thêm, không khớp với những gì code hiện có. Liệt kê lại
đây để **không ai (kể cả Stitch ở lần chạy sau) hiểu nhầm là tính năng đã tồn tại**:

| Stitch mô tả | Thực tế trong code | Trạng thái |
|---|---|---|
| Checksum: XXHash64, MD5, SHA-1, **SHA-256** | `ChecksumAlgorithm` chỉ có `xxh64 \| md5 \| sha1` (`src/types/job.ts`) | ❌ Sai — không có SHA-256, đừng thêm vào UI trừ khi backend hỗ trợ trước |
| Verification chỉ 2 chế độ: "Toàn bộ nội dung & Kích thước (MHL)" / "Khớp Nguồn/Đích" | Thực tế có **3** mode: `transfer` (size-only, nhanh nhất), `source` (hash nguồn khi copy), `sourceAndDestination` (hash cả hai, so khớp — an toàn nhất) | ❌ Sai — thiếu mất 1 mode, đổi nghĩa 2 mode còn lại. Giữ đúng bản mô tả ở mục 3.5/4.1 phía trên |
| "Chọn định dạng Báo cáo (PDF + MHL)" là 1 setting khi tạo job | Không có khái niệm "report format" chọn theo job. MHL được ghi tự động gắn với verification mode; **Reports** (mục 5.3) là tính năng tách biệt — chọn các log *đã xong việc* để xuất PDF/HTML sau đó | ❌ Gộp nhầm 2 tính năng độc lập |
| Tiến trình hiển thị "thời gian còn lại" (ETA) | `ProgressEventPayload` không có trường ETA, chỉ có `bytesPerSec` (`src/types/job.ts`) | ⚠️ Chưa có sẵn — nếu muốn hiển thị, cần tự tính ở client từ `bytesPerSec` + `(totalBytes - bytesCopied)`, không phải dữ liệu có sẵn từ backend |
| Token ví dụ `{YYYY}-{MM}-{DD}_{CAMERA_ID}_{ROLL_ID}` | Token built-in chỉ có `{Source Name} {Counter} {YYYY}{YY}{MM}{DD}{hh}{mm}{ss} {Filename} {File Counter} {File Extension} {File YYYY} {Content YYYY}`. `{CAMERA_ID}`/`{ROLL_ID}` chỉ tồn tại nếu người dùng tự tạo qua **Elements** (custom token) | ⚠️ Gây hiểu lầm là token có sẵn — nếu dùng ví dụ này, phải ghi chú rõ là Element tự định nghĩa |
| "Tên gốc dự án" (1 field) cho path template | Thực tế là **2 field riêng biệt**: Rename template + Folder template (`OrganizeSettings`) | ❌ Sai cấu trúc form |
| Preset dựng sẵn: "DIT Standard", "Fast Copy" | Preset chỉ là cấu hình do người dùng tự đặt tên rồi lưu (`usePresetsStore`) — **không có preset mặc định nào** | ❌ Không tồn tại, chỉ nên dùng làm ví dụ minh họa tên preset, không phải default thật |
| Media Browser có: Filter theo loại file, checkbox chọn từng file (badge "SEL"), nút "Chọn tất cả / Xóa / Thêm vào danh sách Nguồn" | `MediaBrowser.tsx` hiện tại **chỉ xem trước (read-only)**: lưới thumbnail + metadata + nút Close. Không filter, không selection, không action nào | ❌ **Sai lệch lớn nhất** — đây là cả một luồng chức năng mới (cần store/state mới + có thể cả Tauri command mới), không phải chỉ vẽ lại giao diện của tính năng có sẵn |
| Sidebar điều hướng nhiều màn hình (Trang chủ / Xác minh MHL / Báo cáo / Cài đặt) + nút chuyển ngôn ngữ VN/EN | App hiện tại là **1 trang dashboard cuộn dọc duy nhất** (`App.tsx`), không có router multi-screen, và **không có hạ tầng đa ngôn ngữ nào** (không i18n lib, không file dịch) | ❌ Thay đổi kiến trúc, không phải chỉ đổi theme |

**Kết luận:** style system (màu/font/bo góc/elevation/component style) của Stitch đáng
tin cậy và đã merge vào mục 1. Phần kiến trúc màn hình & chức năng thì **giữ nguyên
theo mục 2–8 ở trên** (đã được viết từ source code thật, không đổi) — các ý tưởng mới
của Stitch trong bảng trên (multi-screen nav, đa ngôn ngữ, chọn/lọc file trong Media
Browser, SHA-256, ETA...) là **đề xuất tính năng mới**, chưa tồn tại trong code, và
cần một quyết định riêng (có làm hay không, làm ở đâu — UI lẫn backend) trước khi đưa
vào bất kỳ bản thiết kế hay implementation nào tiếp theo.
