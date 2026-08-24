# MHL interoperability

Nguồn đối chiếu độc lập:

- ASC MITC reference implementation: https://github.com/ascmitc/mhl
- ASC MHL specification repository: https://github.com/ascmitc/mhl-specification
- `xxhash-rust` 0.8.16 official algorithm implementation.

## Đã hỗ trợ

- Legacy MHL 1.1 hiện tại: đọc/ghi và verify.
- XXH64, XXH3-64 và XXH3-128 (`xxh128`) trong copy/verify pipeline.
- Đọc file ASC MHL 2.0 có namespace `urn:ASC:MHL:v2.0`, cấu trúc:
  - `<hashes><hash>`
  - `<path size="..." lastmodificationdate="...">...`
  - `<xxh64>`, `<xxh3>` hoặc `<xxh128>`.

Known vectors được kiểm tra từ reference implementation và xxHash:

- XXH3-64 empty: `2d06800538d394c2`
- XXH3-128 empty: `99aa06d3014798d86001c324468d497f`

## Chưa tự nhận là ASC MHL writer hoàn chỉnh

OffloadKit chưa ghi ASC MHL 2.0 chain-of-custody vì việc đó cần đúng toàn bộ `processinfo`, root content/structure hash, references/chain và external validation. App tiếp tục ghi legacy MHL 1.1 ổn định, nhưng có thể đọc/verify các file ASC MHL 2.0 cấp file.
