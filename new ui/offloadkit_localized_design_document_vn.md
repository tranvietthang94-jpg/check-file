# DESIGN.md — OffloadKit UI Design Brief (Localized VN Edition)

> Tài liệu này mô tả giao diện tiếng Việt hoàn thiện của OffloadKit — một ứng dụng desktop chuyên nghiệp dành cho DIT (Digital Imaging Technician). Giao diện tập trung vào tính kỹ thuật, mật độ thông tin cao, và độ tin cậy tuyệt đối trong việc sao lưu dữ liệu hiện trường.

---

## 1. Nguyên tắc thiết kế (Design Principles)

- **Theme:** Dark Mode (OLED Black). Nền `#131313`, các panel `#1c1b1b`.
- **Bảng màu trạng thái:**
  - **Xanh lá cây (`#19ed60`):** Trạng thái thành công, nút hành động chính, tiến trình hoàn tất (Đã xác minh).
  - **Đỏ:** Lỗi nghiêm trọng, hủy bỏ (Thất bại).
  - **Cam/Vàng:** Cảnh báo, Media bị hỏng (Broken Media).
  - **Trắng/Xám:** Văn bản thông tin, các trạng thái chờ.
- **Typography:** 
  - **Sans-serif (Hanken Grotesk):** Cho các nhãn (labels) và văn bản thông thường.
  - **Monospace:** Bắt buộc cho đường dẫn (path), mã checksum, và các token template.
- **Mật độ (Density):** Rất cao. Tối ưu không gian để hiển thị nhiều thông tin nhất có thể trên một màn hình mà không cần cuộn nhiều.

---

## 2. Kiến trúc các màn hình chính

### 2.1 Bảng điều khiển (Home Dashboard VN)
Đây là màn hình trung tâm, được chia thành các khu vực chức năng:
- **Cột Trái (Sidebar):** Điều hướng chính với các mục: Trang chủ, Xác minh MHL, Báo cáo, Cài đặt. Cuối cùng là nút chuyển đổi ngôn ngữ (VN/EN) và Trạng thái hệ thống.
- **Bảng Ổ đĩa hiện có:** Danh sách các ổ đĩa và thẻ nhớ được cắm vào. Mỗi thẻ hiển thị tên, dung lượng trống và nút "+" để gán làm Nguồn hoặc Đích.
- **Thiết lập công việc:** Form cấu hình nhanh cho một lượt copy. 
  - Chọn Nguồn và Đích (hiển thị đường dẫn monospace).
  - Chọn thuật toán Xác minh (ví dụ: XXHash64).
  - Chọn định dạng Báo cáo (ví dụ: PDF + MHL).
  - Nút "BẮT ĐẦU OFFLOAD" màu xanh lá cây rực rỡ ở dưới cùng.
- **Tiến trình đang chạy:** Theo dõi Real-time các job đang copy. Hiển thị Progress bar màu xanh lá, tốc độ truyền tải (MB/s), thời gian còn lại và tên file hiện tại.
- **Lịch sử gần đây:** Bảng log các job đã xong với các cột: Nguồn, Đích đến, Kích thước, Trạng thái (badge ĐÃ XÁC MINH màu xanh hoặc THẤT BẠI màu đỏ).

### 2.2 Cài đặt Hệ thống (System Configuration VN)
Cấu hình chi tiết các tham số cốt lõi:
- **Phạm vi xác minh:** 
  - *Toàn bộ nội dung & Kích thước (MHL):* Chế độ an toàn nhất, tạo danh sách băm đầy đủ.
  - *Khớp Nguồn/Đích:* Chỉ kiểm tra sự tồn tại và kích thước file, nhanh hơn.
- **Thuật toán Checksum:** Chọn giữa XXHash64 (mặc định), MD5, SHA-1, SHA-256.
- **Template đường dẫn:** 
  - *Tên gốc dự án:* Ô nhập văn bản.
  - *Cấu trúc thư mục:* Dùng token như `{YYYY}-{MM}-{DD}_{CAMERA_ID}_{ROLL_ID}`.
  - *Xem trước đường dẫn:* Render trực tiếp đường dẫn sẽ tạo ra (màu xanh lá monospace).
- **Preset:** Các cấu hình lưu sẵn để áp dụng nhanh (DIT Standard, Fast Copy).

### 2.3 Duyệt Nguồn Media (Media Browser VN)
Cửa sổ modal dùng để chọn lọc file trước khi copy:
- **Khu vực Vị trí:** Danh sách các thư mục/ổ đĩa nguồn.
- **Bộ lọc (Filter):** Checkbox lọc theo loại file (Video .R3D, Âm thanh .WAV...).
- **Lưới hiển thị:** Các ô thumbnail 16:9 kèm metadata chi tiết bên dưới:
  - Độ phân giải (8K R3D), FPS (23.98), Kích thước, Timecode.
  - Badge "SEL" màu xanh lá cho các file đã chọn.
- **Thanh công cụ dưới cùng:** Nút "Chọn tất cả", "Xóa", và nút hành động "THÊM VÀO DANH SÁCH NGUỒN".

---

## 3. Quy chuẩn thuật ngữ Việt hóa (Localization Glossary)

| Tiếng Anh | Tiếng Việt |
|---|---|
| Dashboard | Bảng điều khiển |
| Volumes / Disks | Ổ đĩa hiện có |
| Transfers | Tiến trình |
| Offload | Truyền tải / Sao lưu |
| Verification | Xác minh |
| Checksum | Mã băm / Kiểm tra toàn vẹn |
| Destination | Đích đến |
| Source | Nguồn |
| MHL (Media Hash List) | Danh sách băm Media |
| Project Root | Tên gốc dự án |
| Folder Structure | Cấu trúc thư mục |
| Verified | Đã xác minh |
| Failed | Thất bại |
| Queued | Đang chờ |

---

## 4. Ghi chú kỹ thuật cho Code
- Toàn bộ border sử dụng 1px solid `#3a3939`.
- Các nút hành động chính (Primary) sử dụng background `#19ed60`, text `#131313` (bold).
- Trạng thái hover trên danh sách: background `#2c2b2b`.
- Font Monospace: `JetBrains Mono` hoặc `Roboto Mono` cho các trường dữ liệu kỹ thuật.
