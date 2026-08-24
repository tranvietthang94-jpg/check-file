# Automatic Repair Planner — Design

## Mục tiêu

Khi một file trong transfer/MHL bị mismatch, thiếu hoặc sai kích thước, OffloadKit tự tìm các bản ứng viên tốt trên Source gốc và các Destination sibling đã được xác minh. Ứng dụng chỉ trình bày kế hoạch; không thay file cho đến khi người dùng xác nhận rõ ràng.

## Phạm vi tối giản

- Tái sử dụng MHL parser, checksum verifier và `repair_mhl_entry` hiện có.
- Tìm ứng viên theo cùng `relativePath` trong danh sách root được cung cấp từ transfer log của cùng Source hoặc từ các root người dùng chọn.
- Chỉ chấp nhận ứng viên có kích thước và checksum khớp MHL đích.
- Xếp Source gốc trước, sau đó Destination sibling; ứng viên không xác minh được bị loại.
- Hiển thị: file lỗi, trạng thái lỗi, nguồn ứng viên, thuật toán/checksum và nút xác nhận.
- Khi xác nhận, dùng repair hiện có: giữ `.ofkit-corrupt` evidence duy nhất, atomic replace, rồi verify lại MHL.
- Không auto-repair ngầm; không xóa evidence; không chọn ứng viên chỉ dựa trên tên/kích thước.

## Kiến trúc

### Backend

Thêm hai command nhỏ:

1. `plan_mhl_repair(mhl_path, relative_path, candidate_roots)`
   - Parse entry MHL cần sửa.
   - Chuẩn hóa và kiểm tra `relative_path` không thoát root.
   - Với từng root: tạo candidate path an toàn, từ chối symlink/junction, đọc kích thước và hash theo thuật toán MHL.
   - Trả danh sách ứng viên đã xác minh; không mutation.

2. Giữ nguyên `repair_mhl_entry(...)` cho bước thực thi đã có approval.

Không tạo repair engine/framework mới.

### Frontend

- `MhlVerifyPanel` đổi nút **Sửa** thành mở Repair Plan.
- Repair Plan tự gom candidate roots từ các transfer log có cùng source/destination liên quan; người dùng có thể thêm một folder thủ công nếu chưa tìm thấy.
- Chỉ enable **Xác nhận sửa** khi có ứng viên verified được chọn.
- Sau sửa, thay report hiện tại bằng kết quả verify mới.

## Luồng hoạt động

1. Người dùng Verify MHL.
2. Chọn **Lập kế hoạch sửa** trên file lỗi.
3. App tìm ứng viên và hash từng file.
4. Nếu không có ứng viên hợp lệ: báo rõ và cho chọn thêm folder.
5. Nếu có: hiển thị ứng viên verified; mặc định chọn ứng viên ưu tiên cao nhất.
6. Người dùng bấm xác nhận.
7. Backend giữ evidence, atomic replace, verify lại.
8. UI hiển thị kết quả mới; nếu vẫn mismatch, báo lỗi và không tuyên bố thành công.

## An toàn và lỗi

- Candidate path phải nằm dưới candidate root sau normalization/canonicalization.
- Từ chối candidate hoặc destination đi qua symlink/junction.
- Không mutation trong bước plan.
- Approval bắt buộc trong bước execute.
- Candidate phải khớp checksum MHL; size-only không đủ.
- Evidence không ghi đè file evidence cũ.
- Nếu replace/rollback lỗi, trả cả hai lỗi như logic repair hiện tại.
- Empty/loading/error/success states rõ ràng.

## Kiểm thử

### Rust

- Planner tìm đúng Source candidate khớp checksum.
- Planner tìm Destination sibling khi Source hỏng/mất.
- Loại candidate sai checksum dù cùng size.
- Từ chối traversal và symlink/junction.
- Không ứng viên trả danh sách rỗng, không mutation.
- Execute vẫn yêu cầu approval, giữ evidence và verify lại.

### Playwright

- File lỗi mở Repair Plan.
- Không có candidate thì nút confirm disabled.
- Candidate verified hiển thị checksum/source và confirm được.
- Cancel không mutation.
- Sau repair thành công report chuyển sang `Đã xác minh`.

## Không làm trong phase này

- Tự động sửa không cần xác nhận.
- Repair hàng loạt một lần.
- Hệ thống rules/priority tùy biến.
- Theo dõi chain-of-custody ASC MHL.
- Network/cloud source.
